use crate::deeplink::{
    import_mcp_from_deeplink, import_prompt_from_deeplink, import_provider_from_deeplink,
    import_skill_from_deeplink, parse_deeplink_url, validate_deeplink_request,
    DeepLinkImportRequest,
};
use crate::store::AppState;
use tauri::State;

const DEEPLINK_PARSE_ERROR: &str = "Deep link could not be parsed";
const DEEPLINK_OPERATION_ERROR: &str = "Deep link request could not be processed";

fn validate_ipc_request(request: &DeepLinkImportRequest) -> Result<(), String> {
    validate_deeplink_request(request).map_err(|_| DEEPLINK_OPERATION_ERROR.to_string())
}

/// Parse a deep link URL and return the parsed request for frontend confirmation
#[tauri::command]
pub fn parse_deeplink(url: String) -> Result<DeepLinkImportRequest, String> {
    // A custom-protocol URL can carry an API key. Keep both its contents and
    // parser details out of the renderer-visible error and ordinary logs.
    log::info!("Parsing deep link URL");
    parse_deeplink_url(&url).map_err(|_| {
        log::warn!("Rejected deep link URL at the validation boundary");
        DEEPLINK_PARSE_ERROR.to_string()
    })
}

/// Merge configuration from Base64/URL into a deep link request
/// This is used by the frontend to show the complete configuration in the confirmation dialog
#[tauri::command]
pub fn merge_deeplink_config(
    request: DeepLinkImportRequest,
) -> Result<DeepLinkImportRequest, String> {
    log::info!("Merging configuration for deep link request");
    validate_ipc_request(&request)?;
    let merged = crate::deeplink::parse_and_merge_config(&request)
        .map_err(|_| DEEPLINK_OPERATION_ERROR.to_string())?;
    validate_ipc_request(&merged)?;
    Ok(merged)
}

/// Import a provider from a deep link request (legacy, kept for compatibility)
#[tauri::command]
pub fn import_from_deeplink(
    state: State<AppState>,
    request: DeepLinkImportRequest,
) -> Result<String, String> {
    log::info!("Importing provider from deep link");
    validate_ipc_request(&request)?;

    let provider_id = import_provider_from_deeplink(&state, request)
        .map_err(|_| DEEPLINK_OPERATION_ERROR.to_string())?;

    log::info!("Imported provider from deep link");

    Ok(provider_id)
}

/// Import resource from a deep link request (unified handler)
#[tauri::command]
pub async fn import_from_deeplink_unified(
    state: State<'_, AppState>,
    request: DeepLinkImportRequest,
) -> Result<serde_json::Value, String> {
    log::info!("Importing resource from deep link");
    validate_ipc_request(&request)?;

    match request.resource.as_str() {
        "provider" => {
            let provider_id = import_provider_from_deeplink(&state, request)
                .map_err(|_| DEEPLINK_OPERATION_ERROR.to_string())?;
            Ok(serde_json::json!({
                "type": "provider",
                "id": provider_id
            }))
        }
        "prompt" => {
            let prompt_id = import_prompt_from_deeplink(&state, request)
                .map_err(|_| DEEPLINK_OPERATION_ERROR.to_string())?;
            Ok(serde_json::json!({
                "type": "prompt",
                "id": prompt_id
            }))
        }
        "mcp" => {
            let result = import_mcp_from_deeplink(&state, request)
                .map_err(|_| DEEPLINK_OPERATION_ERROR.to_string())?;
            // Add type field to the result
            Ok(serde_json::json!({
                "type": "mcp",
                "importedCount": result.imported_count,
                "importedIds": result.imported_ids,
                "failed": result.failed
            }))
        }
        "skill" => {
            let skill_key = import_skill_from_deeplink(&state, request)
                .map_err(|_| DEEPLINK_OPERATION_ERROR.to_string())?;
            Ok(serde_json::json!({
                "type": "skill",
                "key": skill_key
            }))
        }
        _ => Err(DEEPLINK_OPERATION_ERROR.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_deeplink, validate_ipc_request, DeepLinkImportRequest};

    #[test]
    fn parse_command_does_not_return_a_link_or_credential_on_failure() {
        let api_key = "sk-deeplink-command-secret";
        let url =
            format!("fyagent://v1/import?resource=provider&app=claude&name=%00&apiKey={api_key}");

        let error = parse_deeplink(url.clone()).expect_err("control character must be rejected");

        assert_eq!(error, "Deep link could not be parsed");
        assert!(!error.contains(api_key));
        assert!(!error.contains(&url));
    }

    #[test]
    fn direct_ipc_request_cannot_bypass_the_credential_safe_validation_boundary() {
        let api_key = "sk-direct-ipc-secret";
        let request = DeepLinkImportRequest {
            version: "v1".to_string(),
            resource: "provider".to_string(),
            api_key: Some(format!("{api_key}\u{0000}")),
            ..DeepLinkImportRequest::default()
        };

        let error = validate_ipc_request(&request).expect_err("control character must be rejected");

        assert_eq!(error, "Deep link request could not be processed");
        assert!(!error.contains(api_key));
    }
}
