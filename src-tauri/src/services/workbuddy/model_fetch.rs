//! Constrained WorkBuddy model-list transport.
//!
//! This deliberately does not reuse the provider model fetcher: WorkBuddy has
//! one canonical endpoint, permits an explicit no-key mode, preserves upstream
//! order, and must enforce its own redirect and response-size boundary.

use std::{collections::HashSet, time::Duration};

use futures::StreamExt;
use reqwest::{header, redirect::Policy, Client, Response};
use serde_json::Value;
use url::Url;

use super::{
    error::{redact_response_summary, WorkBuddyError, WorkBuddyErrorCode},
    types::{FetchWorkBuddyModelsRequest, FetchWorkBuddyModelsResult},
    url::{normalize_workbuddy_base_url, NormalizedWorkBuddyUrl},
};

const FETCH_TIMEOUT: Duration = Duration::from_secs(15);
const MAX_RESPONSE_BYTES: usize = 2 * 1024 * 1024;
const MAX_ERROR_SUMMARY_BYTES: usize = 4 * 1024;
const MAX_REDIRECTS: usize = 3;
const MAX_MODELS: usize = 1_000;
const GENERIC_UPSTREAM_HTTP_ERROR_SUMMARY: &str = "The server returned an HTTP error response.";

pub(crate) async fn fetch_workbuddy_models(
    request: FetchWorkBuddyModelsRequest,
) -> Result<FetchWorkBuddyModelsResult, WorkBuddyError> {
    let normalized = normalize_workbuddy_base_url(&request.base_url)?;
    if request.api_key.trim().is_empty() && !request.allow_no_api_key {
        return Err(WorkBuddyError::new(WorkBuddyErrorCode::ApiKeyRequired));
    }

    let client = build_workbuddy_client()?;
    fetch_with_timeout(&client, &normalized, &request.api_key, FETCH_TIMEOUT).await
}

async fn fetch_with_timeout(
    client: &Client,
    normalized: &NormalizedWorkBuddyUrl,
    api_key: &str,
    timeout: Duration,
) -> Result<FetchWorkBuddyModelsResult, WorkBuddyError> {
    tokio::time::timeout(timeout, fetch_with_client(client, normalized, api_key))
        .await
        .map_err(|_| WorkBuddyError::new(WorkBuddyErrorCode::FetchTimeout))?
}

async fn fetch_with_client(
    client: &Client,
    normalized: &NormalizedWorkBuddyUrl,
    api_key: &str,
) -> Result<FetchWorkBuddyModelsResult, WorkBuddyError> {
    let mut current_url = normalized.models_url.clone();
    let mut redirects = 0usize;

    loop {
        let mut request = client.get(current_url.clone());
        if !api_key.trim().is_empty() {
            request = request.bearer_auth(api_key);
        }

        let response = request.send().await.map_err(|_| {
            WorkBuddyError::new(WorkBuddyErrorCode::FetchHttpError)
                .with_redacted_summary("The model request could not be completed.")
        })?;

        if response.status().is_redirection() {
            if redirects >= MAX_REDIRECTS {
                return Err(WorkBuddyError::new(
                    WorkBuddyErrorCode::FetchRedirectRejected,
                ));
            }
            let redirect_url = redirect_target(&current_url, &response)?;
            if !normalized.origin.matches_url(&redirect_url)
                || (current_url.scheme() == "https" && redirect_url.scheme() != "https")
            {
                return Err(WorkBuddyError::new(
                    WorkBuddyErrorCode::FetchRedirectRejected,
                ));
            }

            redirects += 1;
            current_url = redirect_url;
            continue;
        }

        if !response.status().is_success() {
            let status = response.status().as_u16();
            // A keyed request never forwards any server-controlled error body.
            // Redaction cannot prove that an upstream did not echo a transformed
            // credential, so the only safe response is a stable local summary.
            let summary = if api_key.trim().is_empty() {
                read_error_summary(response).await
            } else {
                GENERIC_UPSTREAM_HTTP_ERROR_SUMMARY.to_string()
            };
            return Err(WorkBuddyError::new(WorkBuddyErrorCode::FetchHttpError)
                .with_http_status(status)
                .with_redacted_summary(summary));
        }

        let body = read_success_body(response).await?;
        return parse_models_response(&body);
    }
}

fn build_workbuddy_client() -> Result<Client, WorkBuddyError> {
    let mut builder = Client::builder()
        .redirect(Policy::none())
        .no_gzip()
        .no_brotli()
        .no_deflate()
        .no_zstd();

    match crate::proxy::http_client::installer_proxy_configuration() {
        Ok(crate::proxy::http_client::InstallerProxyConfiguration::Explicit(proxy_url)) => {
            let proxy = reqwest::Proxy::all(proxy_url.as_str()).map_err(|_| {
                WorkBuddyError::new(WorkBuddyErrorCode::FetchHttpError)
                    .with_redacted_summary("The configured proxy could not be used.")
            })?;
            builder = builder.proxy(proxy);
        }
        Ok(crate::proxy::http_client::InstallerProxyConfiguration::System) => {
            // Preserve the application's normal system-proxy behavior.
        }
        Ok(crate::proxy::http_client::InstallerProxyConfiguration::Direct) => {
            builder = builder.no_proxy();
        }
        Err(()) => {
            return Err(WorkBuddyError::new(WorkBuddyErrorCode::FetchHttpError)
                .with_redacted_summary("The configured proxy is invalid."));
        }
    }

    builder.build().map_err(|_| {
        WorkBuddyError::new(WorkBuddyErrorCode::FetchHttpError)
            .with_redacted_summary("The model request client could not be created.")
    })
}

fn redirect_target(current_url: &Url, response: &Response) -> Result<Url, WorkBuddyError> {
    let location = response
        .headers()
        .get(header::LOCATION)
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| WorkBuddyError::new(WorkBuddyErrorCode::FetchRedirectRejected))?;
    redirect_target_from_location(current_url, location)
}

fn redirect_target_from_location(current_url: &Url, location: &str) -> Result<Url, WorkBuddyError> {
    let target = current_url
        .join(location)
        .map_err(|_| WorkBuddyError::new(WorkBuddyErrorCode::FetchRedirectRejected))?;
    // A same-origin userinfo URL would otherwise pass the origin check while
    // placing attacker-controlled credentials beside our Authorization header.
    // Fragments are never useful to the HTTP request and are rejected so a
    // credential-bearing redirect URL cannot continue through this boundary.
    if !target.username().is_empty() || target.password().is_some() || target.fragment().is_some() {
        return Err(WorkBuddyError::new(
            WorkBuddyErrorCode::FetchRedirectRejected,
        ));
    }
    Ok(target)
}

async fn read_success_body(response: Response) -> Result<Vec<u8>, WorkBuddyError> {
    if response
        .content_length()
        .is_some_and(|length| length > MAX_RESPONSE_BYTES as u64)
    {
        return Err(WorkBuddyError::new(
            WorkBuddyErrorCode::FetchResponseTooLarge,
        ));
    }

    let mut total = 0usize;
    let mut bytes = Vec::new();
    let mut stream = response.bytes_stream();
    while let Some(next) = stream.next().await {
        let chunk = next.map_err(|_| {
            WorkBuddyError::new(WorkBuddyErrorCode::FetchHttpError)
                .with_redacted_summary("The model response could not be read.")
        })?;
        total = total
            .checked_add(chunk.len())
            .ok_or_else(|| WorkBuddyError::new(WorkBuddyErrorCode::FetchResponseTooLarge))?;
        if total > MAX_RESPONSE_BYTES {
            return Err(WorkBuddyError::new(
                WorkBuddyErrorCode::FetchResponseTooLarge,
            ));
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

async fn read_error_summary(response: Response) -> String {
    let mut bytes = Vec::new();
    let mut stream = response.bytes_stream();

    while let Some(next) = stream.next().await {
        let Ok(chunk) = next else {
            break;
        };
        let remaining = MAX_ERROR_SUMMARY_BYTES.saturating_sub(bytes.len());
        if remaining == 0 {
            break;
        }
        let take = remaining.min(chunk.len());
        bytes.extend_from_slice(&chunk[..take]);
        if take < chunk.len() {
            break;
        }
    }

    let body = String::from_utf8_lossy(&bytes);
    let summary = redact_response_summary(&body, "");
    if summary.trim().is_empty() {
        GENERIC_UPSTREAM_HTTP_ERROR_SUMMARY.to_string()
    } else {
        summary
    }
}

fn parse_models_response(bytes: &[u8]) -> Result<FetchWorkBuddyModelsResult, WorkBuddyError> {
    let value: Value = serde_json::from_slice(bytes)
        .map_err(|_| WorkBuddyError::new(WorkBuddyErrorCode::FetchInvalidSchema))?;
    let data = value
        .as_object()
        .and_then(|object| object.get("data"))
        .and_then(Value::as_array)
        .ok_or_else(|| WorkBuddyError::new(WorkBuddyErrorCode::FetchInvalidSchema))?;

    let mut models = Vec::new();
    let mut seen = HashSet::new();
    let mut truncated = false;

    for entry in data {
        let id = entry
            .as_object()
            .and_then(|object| object.get("id"))
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|id| !id.is_empty())
            .ok_or_else(|| WorkBuddyError::new(WorkBuddyErrorCode::FetchInvalidSchema))?;

        if !seen.insert(id.to_string()) {
            continue;
        }
        if models.len() >= MAX_MODELS {
            truncated = true;
            continue;
        }
        models.push(id.to_string());
    }

    Ok(FetchWorkBuddyModelsResult { models, truncated })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{ErrorKind, Read, Write},
        net::{TcpListener, TcpStream},
        sync::{Arc, Mutex},
        thread::{self, JoinHandle},
        time::{Duration, Instant},
    };

    use reqwest::{redirect::Policy, Client};

    // Heavily parallel native CI can schedule the server thread well before
    // the Tokio client future. Keep the shared fixture bounded while allowing
    // for that scheduling skew; product transport timeouts remain independent.
    const TEST_SERVER_ACCEPT_TIMEOUT: Duration = Duration::from_secs(10);

    struct LocalHttpServer {
        base_url: String,
        requests: Arc<Mutex<Vec<String>>>,
        handle: JoinHandle<()>,
    }

    impl LocalHttpServer {
        fn finish(self) -> Vec<String> {
            self.handle.join().expect("local HTTP server thread");
            self.requests.lock().expect("captured requests").clone()
        }
    }

    fn loopback_test_client() -> Client {
        Client::builder()
            .redirect(Policy::none())
            .no_proxy()
            .no_gzip()
            .no_brotli()
            .no_deflate()
            .no_zstd()
            .build()
            .expect("build loopback client")
    }

    fn spawn_scripted_server(responses: Vec<Vec<u8>>, accept_timeout: Duration) -> LocalHttpServer {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback test listener");
        listener
            .set_nonblocking(true)
            .expect("set loopback test listener nonblocking");
        let port = listener.local_addr().expect("read loopback port").port();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let captured_requests = Arc::clone(&requests);
        let handle = thread::spawn(move || {
            for response in responses {
                let Some(mut stream) = accept_connection(&listener, accept_timeout) else {
                    return;
                };
                captured_requests
                    .lock()
                    .expect("captured requests")
                    .push(read_http_request(&mut stream));
                let _ = stream.write_all(&response);
                let _ = stream.flush();
            }
        });

        LocalHttpServer {
            base_url: format!("http://127.0.0.1:{port}"),
            requests,
            handle,
        }
    }

    fn spawn_delayed_server(delay: Duration, response: Vec<u8>) -> LocalHttpServer {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind delayed loopback listener");
        listener
            .set_nonblocking(true)
            .expect("set delayed loopback listener nonblocking");
        let port = listener.local_addr().expect("read delayed port").port();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let captured_requests = Arc::clone(&requests);
        let handle = thread::spawn(move || {
            let Some(mut stream) = accept_connection(&listener, TEST_SERVER_ACCEPT_TIMEOUT) else {
                return;
            };
            captured_requests
                .lock()
                .expect("captured requests")
                .push(read_http_request(&mut stream));
            thread::sleep(delay);
            let _ = stream.write_all(&response);
            let _ = stream.flush();
        });

        LocalHttpServer {
            base_url: format!("http://127.0.0.1:{port}"),
            requests,
            handle,
        }
    }

    fn accept_connection(listener: &TcpListener, timeout: Duration) -> Option<TcpStream> {
        let deadline = Instant::now() + timeout;
        loop {
            match listener.accept() {
                Ok((stream, _)) => return Some(stream),
                Err(error)
                    if matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::Interrupted)
                        && Instant::now() < deadline =>
                {
                    thread::sleep(Duration::from_millis(5));
                }
                Err(_) => return None,
            }
        }
    }

    fn read_http_request(stream: &mut TcpStream) -> String {
        let _ = stream.set_read_timeout(Some(Duration::from_secs(1)));
        let mut bytes = Vec::new();
        let mut chunk = [0u8; 1024];
        while !bytes.windows(4).any(|window| window == b"\r\n\r\n") && bytes.len() < 16 * 1024 {
            match stream.read(&mut chunk) {
                Ok(0) | Err(_) => break,
                Ok(read) => bytes.extend_from_slice(&chunk[..read]),
            }
        }
        String::from_utf8_lossy(&bytes).into_owned()
    }

    fn has_bearer_authorization(request: &str, api_key: &str) -> bool {
        let expected_value = format!("Bearer {api_key}");
        request.lines().any(|line| {
            let Some((name, value)) = line.split_once(':') else {
                return false;
            };
            name.eq_ignore_ascii_case("authorization") && value.trim() == expected_value
        })
    }

    fn http_response(status_line: &str, headers: &[(&str, &str)], body: &[u8]) -> Vec<u8> {
        let mut response = format!(
            "HTTP/1.1 {status_line}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n",
            body.len()
        )
        .into_bytes();
        for (name, value) in headers {
            response.extend_from_slice(name.as_bytes());
            response.extend_from_slice(b": ");
            response.extend_from_slice(value.as_bytes());
            response.extend_from_slice(b"\r\n");
        }
        response.extend_from_slice(b"\r\n");
        response.extend_from_slice(body);
        response
    }

    fn http_response_with_declared_length(status_line: &str, content_length: usize) -> Vec<u8> {
        format!(
            "HTTP/1.1 {status_line}\r\ncontent-type: application/json\r\ncontent-length: {content_length}\r\nconnection: close\r\n\r\n"
        )
        .into_bytes()
    }

    async fn fetch_from_loopback(
        server: &LocalHttpServer,
        api_key: &str,
    ) -> Result<FetchWorkBuddyModelsResult, WorkBuddyError> {
        let normalized =
            normalize_workbuddy_base_url(&server.base_url).expect("normalize loopback URL");
        let client = loopback_test_client();
        fetch_with_client(&client, &normalized, api_key).await
    }

    #[test]
    fn parser_preserves_order_and_case_sensitive_first_occurrences() {
        let result = parse_models_response(
            br#"{
                "data": [
                  {"id":"z-model"},
                  {"id":"A-model"},
                  {"id":"z-model"},
                  {"id":"a-model"}
                ]
              }"#,
        )
        .unwrap();

        assert_eq!(result.models, ["z-model", "A-model", "a-model"]);
        assert!(!result.truncated);
    }

    #[test]
    fn parser_returns_first_thousand_unique_models_with_truncation() {
        let data = (0..1_000)
            .map(|index| serde_json::json!({ "id": format!("model-{index}") }))
            .chain([
                serde_json::json!({ "id": "model-999" }),
                serde_json::json!({ "id": "model-1000" }),
            ])
            .collect::<Vec<_>>();
        let bytes = serde_json::to_vec(&serde_json::json!({ "data": data })).unwrap();

        let result = parse_models_response(&bytes).unwrap();
        assert_eq!(result.models.len(), MAX_MODELS);
        assert_eq!(result.models.first(), Some(&"model-0".to_string()));
        assert_eq!(result.models.last(), Some(&"model-999".to_string()));
        assert!(result.truncated);
    }

    #[test]
    fn parser_validates_entries_after_the_model_cap() {
        let data = (0..=MAX_MODELS)
            .map(|index| serde_json::json!({ "id": format!("model-{index}") }))
            .chain([serde_json::json!({ "id": " " })])
            .collect::<Vec<_>>();
        let bytes = serde_json::to_vec(&serde_json::json!({ "data": data })).unwrap();

        assert_eq!(
            parse_models_response(&bytes).unwrap_err().code(),
            WorkBuddyErrorCode::FetchInvalidSchema
        );
    }

    #[test]
    fn parser_accepts_an_empty_standard_list_but_rejects_invalid_entries() {
        assert_eq!(
            parse_models_response(br#"{"data":[]}"#).unwrap(),
            FetchWorkBuddyModelsResult {
                models: Vec::new(),
                truncated: false,
            }
        );
        for invalid in [
            br#"[]"#.as_slice(),
            br#"{"models":[]}"#.as_slice(),
            br#"{"data":["model-a"]}"#.as_slice(),
            br#"{"data":[{"id":"model-a"},{"id":"  "}]}"#.as_slice(),
            br#"{"data":[{"id":"model-a"},{"id":7}]}"#.as_slice(),
            br#"{"data":[{"id":"model-a"},{"name":"missing-id"}]}"#.as_slice(),
        ] {
            assert_eq!(
                parse_models_response(invalid).unwrap_err().code(),
                WorkBuddyErrorCode::FetchInvalidSchema
            );
        }
    }

    #[test]
    fn redirect_limit_is_three_followed_hops() {
        assert_eq!(MAX_REDIRECTS, 3);
        assert!(reqwest::StatusCode::FOUND.is_redirection());
    }

    #[test]
    fn redirect_target_rejects_userinfo_and_fragments_before_the_next_request() {
        let current = Url::parse("https://api.example.test/v1/models").unwrap();
        assert_eq!(
            redirect_target_from_location(&current, "https://user:pass@api.example.test/next")
                .unwrap_err()
                .code(),
            WorkBuddyErrorCode::FetchRedirectRejected
        );
        assert_eq!(
            redirect_target_from_location(&current, "/next#secret")
                .unwrap_err()
                .code(),
            WorkBuddyErrorCode::FetchRedirectRejected
        );
        assert_eq!(
            redirect_target_from_location(&current, "/next?cursor=safe").unwrap(),
            Url::parse("https://api.example.test/next?cursor=safe").unwrap()
        );
    }

    #[tokio::test]
    async fn empty_api_key_requires_explicit_opt_in_before_transport() {
        let error = fetch_workbuddy_models(FetchWorkBuddyModelsRequest {
            base_url: "https://api.example.test".to_string(),
            api_key: "   ".to_string(),
            allow_no_api_key: false,
        })
        .await
        .expect_err("blank key without opt-in must fail before sending a request");

        assert_eq!(error.code(), WorkBuddyErrorCode::ApiKeyRequired);
    }

    #[tokio::test]
    async fn allowed_empty_api_key_omits_authorization_header() {
        let server = spawn_scripted_server(
            vec![http_response(
                "200 OK",
                &[],
                br#"{"data":[{"id":"local-model"}]}"#,
            )],
            TEST_SERVER_ACCEPT_TIMEOUT,
        );

        let result = fetch_from_loopback(&server, "").await.unwrap();
        assert_eq!(result.models, ["local-model"]);
        let requests = server.finish();
        assert_eq!(requests.len(), 1);
        assert!(
            !requests[0].to_ascii_lowercase().contains("authorization:"),
            "an allowed empty key must not create an Authorization header"
        );
    }

    #[tokio::test]
    async fn keyed_http_errors_never_forward_server_controlled_bodies() {
        let server = spawn_scripted_server(
            vec![http_response(
                "401 Unauthorized",
                &[],
                b"f a k e - m o d e l - k e y was reflected in a nonstandard format",
            )],
            TEST_SERVER_ACCEPT_TIMEOUT,
        );

        let error = fetch_from_loopback(&server, "fake-model-key")
            .await
            .expect_err("non-2xx response must fail");
        let dto = error.to_dto();
        let summary = dto
            .details
            .redacted_summary
            .expect("HTTP failures retain only a redacted summary");
        assert_eq!(dto.code, WorkBuddyErrorCode::FetchHttpError);
        assert_eq!(dto.details.http_status, Some(401));
        assert_eq!(summary, GENERIC_UPSTREAM_HTTP_ERROR_SUMMARY);

        let requests = server.finish();
        assert_eq!(requests.len(), 1);
        assert!(has_bearer_authorization(&requests[0], "fake-model-key"));
    }

    #[tokio::test]
    async fn no_key_http_errors_keep_only_a_bounded_redacted_summary() {
        let server = spawn_scripted_server(
            vec![http_response(
                "403 Forbidden",
                &[],
                b"request rejected at https://user:pass@example.test/path?token=secret",
            )],
            TEST_SERVER_ACCEPT_TIMEOUT,
        );

        let error = fetch_from_loopback(&server, "")
            .await
            .expect_err("non-2xx response must fail");
        let dto = error.to_dto();
        let summary = dto
            .details
            .redacted_summary
            .expect("no-key HTTP failures retain a redacted summary");
        assert_eq!(dto.code, WorkBuddyErrorCode::FetchHttpError);
        assert_eq!(dto.details.http_status, Some(403));
        assert!(!summary.contains("user:pass"));
        assert!(!summary.contains("token=secret"));
        assert!(summary.len() <= MAX_ERROR_SUMMARY_BYTES);

        let requests = server.finish();
        assert_eq!(requests.len(), 1);
        assert!(
            !requests[0].to_ascii_lowercase().contains("authorization:"),
            "no-key mode must not send an Authorization header"
        );
    }

    #[tokio::test]
    async fn oversized_declared_response_is_rejected_before_reading_the_body() {
        let server = spawn_scripted_server(
            vec![http_response_with_declared_length(
                "200 OK",
                MAX_RESPONSE_BYTES + 1,
            )],
            TEST_SERVER_ACCEPT_TIMEOUT,
        );

        let error = fetch_from_loopback(&server, "fake-model-key")
            .await
            .expect_err("declared response larger than 2 MiB must fail closed");
        assert_eq!(error.code(), WorkBuddyErrorCode::FetchResponseTooLarge);
        assert_eq!(server.finish().len(), 1);
    }

    #[tokio::test]
    async fn same_origin_redirect_preserves_authorization_for_each_safe_hop() {
        let server = spawn_scripted_server(
            vec![
                http_response("302 Found", &[("location", "/follow-up")], b""),
                http_response("200 OK", &[], br#"{"data":[{"id":"redirected"}]}"#),
            ],
            TEST_SERVER_ACCEPT_TIMEOUT,
        );

        let result = fetch_from_loopback(&server, "fake-model-key")
            .await
            .unwrap();
        assert_eq!(result.models, ["redirected"]);
        let requests = server.finish();
        assert_eq!(requests.len(), 2);
        assert!(requests[0].starts_with("GET /v1/models HTTP/1.1"));
        assert!(requests[1].starts_with("GET /follow-up HTTP/1.1"));
        assert!(requests
            .iter()
            .all(|request| has_bearer_authorization(request, "fake-model-key")));
    }

    #[tokio::test]
    async fn cross_origin_redirect_is_rejected_before_the_target_is_contacted() {
        let target = spawn_scripted_server(
            vec![http_response(
                "200 OK",
                &[],
                br#"{"data":[{"id":"unexpected"}]}"#,
            )],
            Duration::from_secs(1),
        );
        let location = format!("{}/steal", target.base_url);
        let origin = spawn_scripted_server(
            vec![http_response("302 Found", &[("location", &location)], b"")],
            TEST_SERVER_ACCEPT_TIMEOUT,
        );

        let error = fetch_from_loopback(&origin, "fake-model-key")
            .await
            .expect_err("a port-changing redirect must be rejected");
        assert_eq!(error.code(), WorkBuddyErrorCode::FetchRedirectRejected);
        assert_eq!(origin.finish().len(), 1);
        assert!(
            target.finish().is_empty(),
            "Authorization must never be sent to a cross-origin redirect target"
        );
    }

    #[tokio::test]
    async fn total_fetch_timeout_maps_to_the_stable_timeout_error() {
        let server = spawn_delayed_server(
            Duration::from_millis(100),
            http_response("200 OK", &[], br#"{"data":[{"id":"late"}]}"#),
        );
        let normalized = normalize_workbuddy_base_url(&server.base_url).unwrap();
        let client = loopback_test_client();

        let error = fetch_with_timeout(&client, &normalized, "", Duration::from_millis(10))
            .await
            .expect_err("the injected total timeout must abort the pending fetch");
        assert_eq!(error.code(), WorkBuddyErrorCode::FetchTimeout);
        assert_eq!(server.finish().len(), 1);
    }
}
