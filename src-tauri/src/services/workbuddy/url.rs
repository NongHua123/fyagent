//! One canonical URL parser for both WorkBuddy fetch and save operations.

use url::Url;

use super::error::{WorkBuddyError, WorkBuddyErrorCode};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OriginKey {
    scheme: String,
    host: String,
    effective_port: u16,
}

impl OriginKey {
    pub(crate) fn from_url(url: &Url) -> Result<Self, WorkBuddyError> {
        let host = url.host().ok_or_else(invalid_url)?;
        let effective_port = url.port_or_known_default().ok_or_else(invalid_url)?;
        Ok(Self {
            scheme: url.scheme().to_ascii_lowercase(),
            host: host.to_string().to_ascii_lowercase(),
            effective_port,
        })
    }

    pub(crate) fn matches_url(&self, url: &Url) -> bool {
        Self::from_url(url).is_ok_and(|other| self == &other)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct NormalizedWorkBuddyUrl {
    pub(crate) base_url: Url,
    pub(crate) models_url: Url,
    pub(crate) origin: OriginKey,
}

pub(crate) fn normalize_workbuddy_base_url(
    raw: &str,
) -> Result<NormalizedWorkBuddyUrl, WorkBuddyError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(invalid_url());
    }

    let mut base_url = Url::parse(trimmed).map_err(|_| invalid_url())?;
    if !matches!(base_url.scheme(), "http" | "https")
        || base_url.host().is_none()
        || !base_url.username().is_empty()
        || base_url.password().is_some()
        || base_url.query().is_some()
        || base_url.fragment().is_some()
    {
        return Err(invalid_url());
    }

    normalize_path(&mut base_url);
    let origin = OriginKey::from_url(&base_url)?;

    let mut models_url = base_url.clone();
    let models_path = match models_url.path().trim_end_matches('/') {
        "" | "/" => "/models".to_string(),
        path => format!("{path}/models"),
    };
    models_url.set_path(&models_path);

    Ok(NormalizedWorkBuddyUrl {
        base_url,
        models_url,
        origin,
    })
}

fn normalize_path(url: &mut Url) {
    let mut path = url.path().trim_end_matches('/').to_string();
    if path.is_empty() {
        path = "/".to_string();
    }

    for suffix in ["/chat/completions", "/models", "/responses"] {
        if path.ends_with(suffix) {
            path.truncate(path.len() - suffix.len());
            if path.is_empty() {
                path.push('/');
            }
            break;
        }
    }
    path = path.trim_end_matches('/').to_string();

    // Preserve the original URL serialization, but identify `v1` using the
    // decoded path-segment value. `Url::path_segments()` intentionally keeps
    // percent encoding, so comparing its raw segments would miss `%76%31`.
    let mut path_candidate = url.clone();
    path_candidate.set_path(if path.is_empty() { "/" } else { &path });
    let has_v1_segment = path_candidate
        .path_segments()
        .is_some_and(|mut segments| segments.any(decoded_segment_is_v1));

    let normalized_path = if has_v1_segment {
        if path.is_empty() {
            "/".to_string()
        } else {
            path
        }
    } else if path.is_empty() {
        "/v1".to_string()
    } else {
        format!("{path}/v1")
    };
    url.set_path(&normalized_path);
}

fn decoded_segment_is_v1(segment: &str) -> bool {
    let bytes = segment.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;

    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            if let (Some(high), Some(low)) =
                (hex_value(bytes[index + 1]), hex_value(bytes[index + 2]))
            {
                decoded.push((high << 4) | low);
                index += 3;
                continue;
            }
        }

        decoded.push(bytes[index]);
        index += 1;
    }

    decoded == b"v1"
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn invalid_url() -> WorkBuddyError {
    WorkBuddyError::new(WorkBuddyErrorCode::InvalidUrl)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_expected_base_and_models_urls() {
        let cases = [
            (
                " https://api.example.com ",
                "https://api.example.com/v1",
                "https://api.example.com/v1/models",
            ),
            (
                "https://api.example.com/v1/",
                "https://api.example.com/v1",
                "https://api.example.com/v1/models",
            ),
            (
                "https://gateway.example.com/openai",
                "https://gateway.example.com/openai/v1",
                "https://gateway.example.com/openai/v1/models",
            ),
            (
                "https://gateway.example.com/openai/v1/chat/completions",
                "https://gateway.example.com/openai/v1",
                "https://gateway.example.com/openai/v1/models",
            ),
            (
                "https://example.com/v1/proxy",
                "https://example.com/v1/proxy",
                "https://example.com/v1/proxy/models",
            ),
        ];

        for (input, expected_base, expected_models) in cases {
            let normalized = normalize_workbuddy_base_url(input).unwrap();
            assert_eq!(normalized.base_url.as_str(), expected_base);
            assert_eq!(normalized.models_url.as_str(), expected_models);
        }
    }

    #[test]
    fn strips_only_standard_terminal_endpoints_and_recognizes_v1_segments() {
        let chat =
            normalize_workbuddy_base_url("https://example.test/v1/chat/completions").unwrap();
        assert_eq!(chat.base_url.as_str(), "https://example.test/v1");

        let response = normalize_workbuddy_base_url("https://example.test/a/responses/").unwrap();
        assert_eq!(response.base_url.as_str(), "https://example.test/a/v1");

        let non_matching =
            normalize_workbuddy_base_url("https://v1.example.test/api-v1/v10").unwrap();
        assert_eq!(
            non_matching.base_url.as_str(),
            "https://v1.example.test/api-v1/v10/v1"
        );

        let percent_encoded = normalize_workbuddy_base_url("https://example.test/%76%31").unwrap();
        assert_eq!(
            percent_encoded.base_url.as_str(),
            "https://example.test/%76%31"
        );
        assert_eq!(
            percent_encoded.models_url.as_str(),
            "https://example.test/%76%31/models"
        );

        let encoded_non_matching =
            normalize_workbuddy_base_url("https://example.test/%76%32").unwrap();
        assert_eq!(
            encoded_non_matching.base_url.as_str(),
            "https://example.test/%76%32/v1"
        );
    }

    #[test]
    fn rejects_unsupported_or_credential_bearing_urls() {
        for input in [
            "",
            "file:///tmp/models.json",
            "https://user:pass@example.test/v1",
            "https://example.test/v1?token=secret",
            "https://example.test/v1#fragment",
            "https://",
        ] {
            assert_eq!(
                normalize_workbuddy_base_url(input).unwrap_err().code(),
                WorkBuddyErrorCode::InvalidUrl,
                "input {input:?} should fail closed"
            );
        }
    }

    #[test]
    fn origin_requires_same_scheme_host_and_effective_port() {
        let origin = OriginKey::from_url(&Url::parse("https://EXAMPLE.test/v1").unwrap()).unwrap();
        assert!(origin.matches_url(&Url::parse("https://example.test/other").unwrap()));
        assert!(!origin.matches_url(&Url::parse("http://example.test/other").unwrap()));
        assert!(!origin.matches_url(&Url::parse("https://example.test:444/other").unwrap()));
        assert!(!origin.matches_url(&Url::parse("https://elsewhere.test/other").unwrap()));
    }
}
