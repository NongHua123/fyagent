//! Runtime transport adapters for the constrained installer core.
//!
//! This module does not expose a renderer-facing API. It adapts the dedicated
//! installer HTTP transport to the metadata trait without widening either
//! trust boundary. AppState and platform factory wiring lives in the crate
//! root so this module remains independently testable with the core domain.

use std::sync::Arc;

use futures::future::BoxFuture;
use url::Url;

use super::{
    cancellation::NeverCancelled,
    download::{
        get_with_redirects, HttpTransport, InstallerHttpClientOptions, RedirectRequestError,
        ReqwestInstallerTransport, TransportError, TransportFuture,
    },
    source::{MetadataEndpoint, MetadataFetcher, MetadataResponse},
};

/// Metadata uses the same clean, redirect-disabled installer transport as an
/// artifact download, but keeps endpoint selection in `MetadataEndpoint`.
/// This adapter never accepts a URL from IPC or remote metadata.
pub(crate) struct InstallerMetadataFetcher {
    transport: Arc<dyn HttpTransport>,
}

/// Rebuilds a dedicated, credential-free client for every request using the
/// current global proxy policy. This preserves the app's proxy hot updates
/// without inheriting the global client's redirect or decoding settings.
pub(crate) struct RuntimeInstallerTransport {
    purpose: InstallerTransportPurpose,
    user_agent: String,
}

#[derive(Clone, Copy)]
pub(crate) enum InstallerTransportPurpose {
    Metadata,
    Download,
}

impl RuntimeInstallerTransport {
    pub(crate) fn new(purpose: InstallerTransportPurpose, user_agent: impl Into<String>) -> Self {
        Self {
            purpose,
            user_agent: user_agent.into(),
        }
    }
}

impl HttpTransport for RuntimeInstallerTransport {
    fn get(&self, url: Url) -> TransportFuture<'_> {
        let purpose = self.purpose;
        let user_agent = self.user_agent.clone();
        Box::pin(async move {
            let (proxy_url, bypass_system_proxy) = runtime_proxy_configuration()?;
            let options = match purpose {
                InstallerTransportPurpose::Metadata => {
                    InstallerHttpClientOptions::for_metadata(proxy_url, user_agent)
                }
                InstallerTransportPurpose::Download => {
                    InstallerHttpClientOptions::for_download(proxy_url, user_agent)
                }
            };
            let options = if bypass_system_proxy {
                options.without_system_proxy()
            } else {
                options
            };
            let transport = ReqwestInstallerTransport::new(options).map_err(|_| {
                TransportError::non_retryable("installer HTTP client could not be created")
            })?;
            transport.get(url).await
        })
    }
}

#[cfg(not(test))]
fn runtime_proxy_configuration() -> Result<(Option<Url>, bool), TransportError> {
    match crate::proxy::http_client::installer_proxy_configuration()
        .map_err(|_| TransportError::non_retryable("installer proxy configuration is invalid"))?
    {
        crate::proxy::http_client::InstallerProxyConfiguration::Explicit(url) => {
            Ok((Some(url), false))
        }
        crate::proxy::http_client::InstallerProxyConfiguration::System => Ok((None, false)),
        crate::proxy::http_client::InstallerProxyConfiguration::Direct => Ok((None, true)),
    }
}

// The path-included core contract test does not compile the application's
// proxy subsystem. Its fake transports do not exercise this production glue,
// so use a direct test-only policy and cover the real mapping in the app build.
#[cfg(test)]
fn runtime_proxy_configuration() -> Result<(Option<Url>, bool), TransportError> {
    Ok((None, false))
}

impl InstallerMetadataFetcher {
    pub(crate) fn new(transport: Arc<dyn HttpTransport>) -> Self {
        Self { transport }
    }
}

impl MetadataFetcher for InstallerMetadataFetcher {
    fn fetch<'a>(
        &'a self,
        endpoint: MetadataEndpoint,
    ) -> BoxFuture<'a, Result<MetadataResponse, TransportError>> {
        Box::pin(async move {
            let url = Url::parse(endpoint.url()).map_err(|_| {
                TransportError::non_retryable("the fixed metadata endpoint is invalid")
            })?;
            // The fixed AgentsMirror metadata endpoints currently redirect to
            // their R2 mirror. Follow only the same bounded HTTPS redirect
            // policy used for package downloads; release metadata still never
            // supplies a request URL to this adapter or to the renderer.
            let response = get_with_redirects(self.transport.as_ref(), url, &NeverCancelled)
                .await
                .map_err(metadata_redirect_error)?;
            if !(200..300).contains(&response.status) {
                return Err(metadata_status_error(response.status));
            }

            Ok(MetadataResponse {
                content_length: response.content_length,
                body: response.body,
            })
        })
    }
}

fn metadata_redirect_error(error: RedirectRequestError) -> TransportError {
    match error {
        RedirectRequestError::Transport(error) => error,
        RedirectRequestError::Redirect(_) => {
            TransportError::redirect_rejected("metadata redirect did not meet the installer policy")
        }
        // Metadata itself is raced with the service-owned cancellation token.
        // `NeverCancelled` is used only inside the redirect helper, so this is
        // defensive and cannot expose a cancellation capability at this layer.
        RedirectRequestError::Cancelled => {
            TransportError::non_retryable("metadata redirect request was unexpectedly cancelled")
        }
    }
}

fn metadata_status_error(status: u16) -> TransportError {
    let diagnostic = format!("metadata request returned HTTP status {status}");
    if status == 408 || status == 429 || (500..600).contains(&status) {
        TransportError::retryable(diagnostic)
    } else {
        // Non-transient metadata statuses are terminal. Redirects with a
        // location have already gone through the bounded policy above.
        TransportError::non_retryable(diagnostic)
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::VecDeque, sync::Mutex};

    use bytes::Bytes;
    use futures::{stream, StreamExt};

    use super::*;
    use crate::codex_desktop::download::{BodyStream, TransportFuture, TransportResponse};

    struct FakeTransport {
        responses: Mutex<VecDeque<Result<TransportResponse, TransportError>>>,
        requests: Mutex<Vec<Url>>,
    }

    impl FakeTransport {
        fn with_response(response: TransportResponse) -> Self {
            Self::with_responses([response])
        }

        fn with_responses(responses: impl IntoIterator<Item = TransportResponse>) -> Self {
            Self {
                responses: Mutex::new(responses.into_iter().map(Ok).collect()),
                requests: Mutex::new(Vec::new()),
            }
        }
    }

    impl HttpTransport for FakeTransport {
        fn get(&self, url: Url) -> TransportFuture<'_> {
            self.requests.lock().unwrap().push(url);
            let response = self
                .responses
                .lock()
                .unwrap()
                .pop_front()
                .expect("test transport is requested only as expected");
            Box::pin(async move { response })
        }
    }

    fn response(
        status: u16,
        location: Option<&str>,
        content_length: Option<u64>,
        body: &[u8],
    ) -> TransportResponse {
        let body: BodyStream = Box::pin(stream::iter(vec![Ok(Bytes::copy_from_slice(body))]));
        TransportResponse {
            status,
            location: location.map(str::to_owned),
            content_length,
            retry_after: None,
            body,
        }
    }

    #[tokio::test]
    async fn metadata_fetcher_uses_only_fixed_endpoint_and_preserves_streaming_body() {
        let transport = Arc::new(FakeTransport::with_response(response(
            200,
            None,
            Some(3),
            b"abc",
        )));
        let fetcher = InstallerMetadataFetcher::new(transport.clone());

        let metadata = fetcher.fetch(MetadataEndpoint::Checksums).await.unwrap();
        let body = metadata.body.collect::<Vec<_>>().await;

        assert_eq!(metadata.content_length, Some(3));
        assert_eq!(body, vec![Ok(Bytes::from_static(b"abc"))]);
        assert_eq!(
            transport.requests.lock().unwrap().as_slice(),
            &[Url::parse(MetadataEndpoint::Checksums.url()).unwrap()]
        );
    }

    #[tokio::test]
    async fn metadata_fetcher_follows_the_bounded_https_redirect_policy() {
        let redirected = Url::parse("https://codexapp-r2.agentsmirror.com/latest/checksums")
            .expect("redirect fixture URL must parse");
        let transport = Arc::new(FakeTransport::with_responses([
            response(302, Some(redirected.as_str()), None, b""),
            response(200, None, Some(3), b"abc"),
        ]));
        let fetcher = InstallerMetadataFetcher::new(transport.clone());

        let metadata = fetcher.fetch(MetadataEndpoint::Checksums).await.unwrap();
        let body = metadata.body.collect::<Vec<_>>().await;

        assert_eq!(metadata.content_length, Some(3));
        assert_eq!(body, vec![Ok(Bytes::from_static(b"abc"))]);
        assert_eq!(
            transport.requests.lock().unwrap().as_slice(),
            &[
                Url::parse(MetadataEndpoint::Checksums.url()).unwrap(),
                redirected
            ]
        );
    }

    #[tokio::test]
    async fn metadata_fetcher_rejects_an_insecure_redirect_before_requesting_it() {
        let transport = Arc::new(FakeTransport::with_response(response(
            302,
            Some("http://untrusted.example/metadata"),
            None,
            b"",
        )));
        let fetcher = InstallerMetadataFetcher::new(transport.clone());

        let error = match fetcher.fetch(MetadataEndpoint::Manifest).await {
            Err(error) => error,
            Ok(_) => panic!("insecure metadata redirect must be rejected"),
        };

        assert!(!error.is_retryable());
        assert!(error.is_redirect_rejected());
        assert_eq!(
            transport.requests.lock().unwrap().as_slice(),
            &[Url::parse(MetadataEndpoint::Manifest.url()).unwrap()]
        );
    }

    #[test]
    fn metadata_statuses_only_retry_transient_failures() {
        let unavailable = metadata_status_error(404);
        assert!(!unavailable.is_retryable());

        let throttled = metadata_status_error(429);
        assert!(throttled.is_retryable());

        let server_error = metadata_status_error(503);
        assert!(server_error.is_retryable());
    }
}
