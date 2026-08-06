//! Deep link URL parser
//!
//! Parses fyagent:// URLs into DeepLinkImportRequest structures.

use super::utils::validate_url;
use super::DeepLinkImportRequest;
use crate::error::AppError;
use std::collections::{HashMap, HashSet};
use url::Url;

/// The protocol is commonly delivered through a command line or browser URL.
/// Bound its total untrusted envelope while leaving room for current Base64
/// configuration payloads.
pub(super) const MAX_DEEPLINK_URL_BYTES: usize = 64 * 1024;
/// Current provider links can carry 23 fields. The remaining slots are a
/// compatibility buffer, not an invitation to accept unbounded input.
pub(super) const MAX_DEEPLINK_QUERY_PARAMETERS: usize = 32;
pub(super) const MAX_DEEPLINK_PARAMETER_KEY_BYTES: usize = 64;
pub(super) const MAX_DEEPLINK_PARAMETER_VALUE_BYTES: usize = 8 * 1024;
/// These values are Base64-encoded documents/scripts rather than ordinary
/// display fields. They retain a larger, still bounded, envelope.
pub(super) const MAX_DEEPLINK_EMBEDDED_PAYLOAD_BYTES: usize = 48 * 1024;

const EMBEDDED_PAYLOAD_PARAMETERS: &[&str] = &["content", "config", "usageScript"];

fn contains_control_character(value: &str) -> bool {
    value.chars().any(char::is_control)
}

/// Reject a percent-encoded sequence which survives the URL parser's one
/// decoding pass (for example `%252F` -> `%2F`). No consumer is allowed to
/// decode a second time, so accepting such input only creates ambiguity.
fn contains_second_percent_encoding(value: &str) -> bool {
    value.as_bytes().windows(3).any(|bytes| {
        bytes[0] == b'%' && bytes[1].is_ascii_hexdigit() && bytes[2].is_ascii_hexdigit()
    })
}

fn maximum_value_bytes(key: &str) -> usize {
    if EMBEDDED_PAYLOAD_PARAMETERS.contains(&key) {
        MAX_DEEPLINK_EMBEDDED_PAYLOAD_BYTES
    } else {
        MAX_DEEPLINK_PARAMETER_VALUE_BYTES
    }
}

/// Revalidate a deserialized request before an IPC caller can merge or import
/// it.  The URL parser already enforces these limits for protocol activation,
/// but renderer IPC can otherwise construct this DTO directly and bypass the
/// wire-level boundary.
///
/// The validation deliberately reports only generic reasons: callers must
/// never receive an API key, a full deep link, or a nested configuration value
/// as part of a failure message.
pub fn validate_deeplink_request(request: &DeepLinkImportRequest) -> Result<(), AppError> {
    let envelope = serde_json::to_vec(request)
        .map_err(|_| AppError::InvalidInput("Deep link request cannot be encoded".to_string()))?;
    if envelope.len() > MAX_DEEPLINK_URL_BYTES {
        return Err(AppError::InvalidInput(
            "Deep link request exceeds its maximum length".to_string(),
        ));
    }

    if request.activation_approved.is_some() && request.resource != "provider" {
        return Err(AppError::InvalidInput(
            "Deep link activation approval is not valid for this resource".to_string(),
        ));
    }
    if request.activation_approved == Some(true) && request.enabled != Some(true) {
        return Err(AppError::InvalidInput(
            "Deep link activation approval has no requested activation".to_string(),
        ));
    }

    let fields: [(&str, Option<&str>); 25] = [
        ("version", Some(request.version.as_str())),
        ("resource", Some(request.resource.as_str())),
        ("app", request.app.as_deref()),
        ("name", request.name.as_deref()),
        ("homepage", request.homepage.as_deref()),
        ("endpoint", request.endpoint.as_deref()),
        ("apiKey", request.api_key.as_deref()),
        ("icon", request.icon.as_deref()),
        ("model", request.model.as_deref()),
        ("notes", request.notes.as_deref()),
        ("haikuModel", request.haiku_model.as_deref()),
        ("sonnetModel", request.sonnet_model.as_deref()),
        ("opusModel", request.opus_model.as_deref()),
        ("content", request.content.as_deref()),
        ("description", request.description.as_deref()),
        ("apps", request.apps.as_deref()),
        ("repo", request.repo.as_deref()),
        ("directory", request.directory.as_deref()),
        ("branch", request.branch.as_deref()),
        ("config", request.config.as_deref()),
        ("configFormat", request.config_format.as_deref()),
        ("configUrl", request.config_url.as_deref()),
        ("usageScript", request.usage_script.as_deref()),
        ("usageApiKey", request.usage_api_key.as_deref()),
        ("usageBaseUrl", request.usage_base_url.as_deref()),
    ];

    for (key, value) in fields {
        if let Some(value) = value {
            if value.len() > maximum_value_bytes(key) {
                return Err(AppError::InvalidInput(
                    "Deep link request field exceeds its maximum length".to_string(),
                ));
            }
            if contains_control_character(value) || contains_second_percent_encoding(value) {
                return Err(AppError::InvalidInput(
                    "Deep link request contains unsupported encoded data".to_string(),
                ));
            }
        }
    }

    for value in [
        request.usage_access_token.as_deref(),
        request.usage_user_id.as_deref(),
    ]
    .into_iter()
    .flatten()
    {
        if value.len() > MAX_DEEPLINK_PARAMETER_VALUE_BYTES
            || contains_control_character(value)
            || contains_second_percent_encoding(value)
        {
            return Err(AppError::InvalidInput(
                "Deep link request contains an invalid field".to_string(),
            ));
        }
    }

    Ok(())
}

/// Decode query pairs exactly once and validate their wire-level envelope
/// before any resource parser observes them. The error messages deliberately
/// avoid interpolating keys or values because a link can carry credentials.
fn parse_query_parameters(url: &Url) -> Result<HashMap<String, String>, AppError> {
    let mut params = HashMap::new();
    let mut seen_keys = HashSet::new();

    for (index, (key, value)) in url.query_pairs().into_owned().enumerate() {
        if index >= MAX_DEEPLINK_QUERY_PARAMETERS {
            return Err(AppError::InvalidInput(
                "Deep link has too many parameters".to_string(),
            ));
        }

        if key.is_empty() || key.len() > MAX_DEEPLINK_PARAMETER_KEY_BYTES {
            return Err(AppError::InvalidInput(
                "Deep link parameter name is invalid".to_string(),
            ));
        }

        if contains_control_character(&key) || contains_control_character(&value) {
            return Err(AppError::InvalidInput(
                "Deep link contains control characters".to_string(),
            ));
        }

        if contains_second_percent_encoding(&key) || contains_second_percent_encoding(&value) {
            return Err(AppError::InvalidInput(
                "Deep link contains a second percent encoding".to_string(),
            ));
        }

        if value.len() > maximum_value_bytes(&key) {
            return Err(AppError::InvalidInput(
                "Deep link parameter value exceeds its maximum length".to_string(),
            ));
        }

        if !seen_keys.insert(key.clone()) {
            return Err(AppError::InvalidInput(
                "Deep link contains duplicate parameters".to_string(),
            ));
        }

        params.insert(key, value);
    }

    Ok(params)
}

/// Parse a fyagent:// URL into a DeepLinkImportRequest
///
/// Expected format:
/// fyagent://v1/import?resource={type}&...
pub fn parse_deeplink_url(url_str: &str) -> Result<DeepLinkImportRequest, AppError> {
    if url_str.len() > MAX_DEEPLINK_URL_BYTES {
        return Err(AppError::InvalidInput(
            "Deep link URL exceeds its maximum length".to_string(),
        ));
    }

    if contains_control_character(url_str) {
        return Err(AppError::InvalidInput(
            "Deep link contains control characters".to_string(),
        ));
    }

    // Parse URL
    let url = Url::parse(url_str)
        .map_err(|_| AppError::InvalidInput("Malformed deep link URL".to_string()))?;

    // Validate scheme
    let scheme = url.scheme();
    if scheme != "fyagent" {
        return Err(AppError::InvalidInput(
            "Deep link must use the fyagent scheme".to_string(),
        ));
    }

    // Extract version from host
    let version = url
        .host_str()
        .ok_or_else(|| AppError::InvalidInput("Missing version in URL host".to_string()))?
        .to_string();

    // Validate version
    if version != "v1" {
        return Err(AppError::InvalidInput(
            "Unsupported deep link protocol version".to_string(),
        ));
    }

    // Extract path (should be "/import")
    let path = url.path();
    if path != "/import" {
        return Err(AppError::InvalidInput(
            "Unsupported deep link action".to_string(),
        ));
    }

    // Parse query parameters
    let params = parse_query_parameters(&url)?;

    // Extract and validate resource type
    let resource = params
        .get("resource")
        .ok_or_else(|| AppError::InvalidInput("Missing 'resource' parameter".to_string()))?
        .clone();

    // Dispatch to appropriate parser based on resource type
    let request = match resource.as_str() {
        "provider" => parse_provider_deeplink(&params, version, resource),
        "prompt" => parse_prompt_deeplink(&params, version, resource),
        "mcp" => parse_mcp_deeplink(&params, version, resource),
        "skill" => parse_skill_deeplink(&params, version, resource),
        _ => Err(AppError::InvalidInput(
            "Unsupported deep link resource".to_string(),
        )),
    }?;

    validate_deeplink_request(&request)?;
    Ok(request)
}

/// Parse provider deep link parameters
fn parse_provider_deeplink(
    params: &HashMap<String, String>,
    version: String,
    resource: String,
) -> Result<DeepLinkImportRequest, AppError> {
    let app = params
        .get("app")
        .ok_or_else(|| AppError::InvalidInput("Missing 'app' parameter".to_string()))?
        .clone();

    // Validate app type
    if !matches!(
        app.as_str(),
        "claude" | "codex" | "gemini" | "grokbuild" | "opencode" | "openclaw" | "hermes"
    ) {
        return Err(AppError::InvalidInput(
            "Unsupported deep link application".to_string(),
        ));
    }

    let name = params
        .get("name")
        .ok_or_else(|| AppError::InvalidInput("Missing 'name' parameter".to_string()))?
        .clone();

    // Make these optional for config file auto-fill (v3.8+)
    let homepage = params.get("homepage").cloned();
    let endpoint = params.get("endpoint").cloned();
    let api_key = params.get("apiKey").cloned();

    // Validate URLs only if provided
    if let Some(ref hp) = homepage {
        if !hp.is_empty() {
            validate_url(hp, "homepage")?;
        }
    }
    // Validate each endpoint (supports comma-separated multiple URLs)
    if let Some(ref ep) = endpoint {
        for (i, url) in ep.split(',').enumerate() {
            let trimmed = url.trim();
            if !trimmed.is_empty() {
                validate_url(trimmed, &format!("endpoint[{i}]"))?;
            }
        }
    }

    // Extract optional fields
    let model = params.get("model").cloned();
    let notes = params.get("notes").cloned();
    let haiku_model = params.get("haikuModel").cloned();
    let sonnet_model = params.get("sonnetModel").cloned();
    let opus_model = params.get("opusModel").cloned();
    let icon = params
        .get("icon")
        .map(|v| v.trim().to_lowercase())
        .filter(|v| !v.is_empty());
    let config = params.get("config").cloned();
    let config_format = params.get("configFormat").cloned();
    let config_url = params.get("configUrl").cloned();
    let enabled = params.get("enabled").and_then(|v| v.parse::<bool>().ok());

    // Extract usage script fields (v3.9+)
    let usage_enabled = params
        .get("usageEnabled")
        .and_then(|v| v.parse::<bool>().ok());
    let usage_script = params.get("usageScript").cloned();
    let usage_api_key = params.get("usageApiKey").cloned();
    let usage_base_url = params.get("usageBaseUrl").cloned();
    let usage_access_token = params.get("usageAccessToken").cloned();
    let usage_user_id = params.get("usageUserId").cloned();
    let usage_auto_interval = params
        .get("usageAutoInterval")
        .and_then(|v| v.parse::<u64>().ok());

    Ok(DeepLinkImportRequest {
        version,
        resource,
        app: Some(app),
        name: Some(name),
        enabled,
        // A URL can request activation through `enabled`, but it must never
        // be able to manufacture the renderer's explicit approval bit.
        activation_approved: None,
        homepage,
        endpoint,
        api_key,
        icon,
        model,
        notes,
        haiku_model,
        sonnet_model,
        opus_model,
        content: None,
        description: None,
        apps: None,
        repo: None,
        directory: None,
        branch: None,
        config,
        config_format,
        config_url,
        usage_enabled,
        usage_script,
        usage_api_key,
        usage_base_url,
        usage_access_token,
        usage_user_id,
        usage_auto_interval,
    })
}

/// Parse prompt deep link parameters
fn parse_prompt_deeplink(
    params: &HashMap<String, String>,
    version: String,
    resource: String,
) -> Result<DeepLinkImportRequest, AppError> {
    let app = params
        .get("app")
        .ok_or_else(|| AppError::InvalidInput("Missing 'app' parameter for prompt".to_string()))?
        .clone();

    // Validate app type
    if !matches!(
        app.as_str(),
        "claude" | "codex" | "gemini" | "grokbuild" | "opencode" | "openclaw" | "hermes"
    ) {
        return Err(AppError::InvalidInput(
            "Unsupported deep link application".to_string(),
        ));
    }

    let name = params
        .get("name")
        .ok_or_else(|| AppError::InvalidInput("Missing 'name' parameter for prompt".to_string()))?
        .clone();

    let content = params
        .get("content")
        .ok_or_else(|| {
            AppError::InvalidInput("Missing 'content' parameter for prompt".to_string())
        })?
        .clone();

    let description = params.get("description").cloned();
    let enabled = params.get("enabled").and_then(|v| v.parse::<bool>().ok());

    Ok(DeepLinkImportRequest {
        version,
        resource,
        app: Some(app),
        name: Some(name),
        enabled,
        activation_approved: None,
        content: Some(content),
        description,
        icon: None,
        homepage: None,
        endpoint: None,
        api_key: None,
        model: None,
        notes: None,
        haiku_model: None,
        sonnet_model: None,
        opus_model: None,
        apps: None,
        repo: None,
        directory: None,
        branch: None,
        config: None,
        config_format: None,
        config_url: None,
        usage_enabled: None,
        usage_script: None,
        usage_api_key: None,
        usage_base_url: None,
        usage_access_token: None,
        usage_user_id: None,
        usage_auto_interval: None,
    })
}

/// Parse MCP deep link parameters
fn parse_mcp_deeplink(
    params: &HashMap<String, String>,
    version: String,
    resource: String,
) -> Result<DeepLinkImportRequest, AppError> {
    let apps = params
        .get("apps")
        .ok_or_else(|| AppError::InvalidInput("Missing 'apps' parameter for MCP".to_string()))?
        .clone();

    // Validate apps format
    for app in apps.split(',') {
        let trimmed = app.trim();
        if !matches!(
            trimmed,
            "claude"
                | "codex"
                | "gemini"
                | "grokbuild"
                | "grok"
                | "opencode"
                | "openclaw"
                | "hermes"
        ) {
            return Err(AppError::InvalidInput(
                "Unsupported deep link application".to_string(),
            ));
        }
    }

    let config = params
        .get("config")
        .ok_or_else(|| AppError::InvalidInput("Missing 'config' parameter for MCP".to_string()))?
        .clone();

    let enabled = params.get("enabled").and_then(|v| v.parse::<bool>().ok());

    Ok(DeepLinkImportRequest {
        version,
        resource,
        apps: Some(apps),
        enabled,
        activation_approved: None,
        config: Some(config),
        config_format: Some("json".to_string()), // MCP config is always JSON
        app: None,
        name: None,
        icon: None,
        homepage: None,
        endpoint: None,
        api_key: None,
        model: None,
        notes: None,
        haiku_model: None,
        sonnet_model: None,
        opus_model: None,
        content: None,
        description: None,
        repo: None,
        directory: None,
        branch: None,
        config_url: None,
        usage_enabled: None,
        usage_script: None,
        usage_api_key: None,
        usage_base_url: None,
        usage_access_token: None,
        usage_user_id: None,
        usage_auto_interval: None,
    })
}

/// Parse skill deep link parameters
fn parse_skill_deeplink(
    params: &HashMap<String, String>,
    version: String,
    resource: String,
) -> Result<DeepLinkImportRequest, AppError> {
    let repo = params
        .get("repo")
        .ok_or_else(|| AppError::InvalidInput("Missing 'repo' parameter for skill".to_string()))?
        .clone();

    // Validate repo format (should be "owner/name")
    if !repo.contains('/') || repo.split('/').count() != 2 {
        return Err(AppError::InvalidInput(
            "Invalid deep link repository".to_string(),
        ));
    }

    let directory = params.get("directory").cloned();
    let branch = params.get("branch").cloned();

    Ok(DeepLinkImportRequest {
        version,
        resource,
        repo: Some(repo),
        directory,
        branch,
        icon: None,
        app: Some("claude".to_string()), // Skills are Claude-only
        name: None,
        enabled: None,
        activation_approved: None,
        homepage: None,
        endpoint: None,
        api_key: None,
        model: None,
        notes: None,
        haiku_model: None,
        sonnet_model: None,
        opus_model: None,
        content: None,
        description: None,
        apps: None,
        config: None,
        config_format: None,
        config_url: None,
        usage_enabled: None,
        usage_script: None,
        usage_api_key: None,
        usage_base_url: None,
        usage_access_token: None,
        usage_user_id: None,
        usage_auto_interval: None,
    })
}
