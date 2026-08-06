//! Stable, privacy-safe WorkBuddy IPC data-transfer objects.
//!
//! WorkBuddy intentionally remains outside the provider/AppType domain. These
//! types expose only the data that the page needs to render its isolated
//! configuration experience; model documents and credentials never cross IPC.

use std::fmt;

use serde::{Deserialize, Serialize};

/// The persisted shape of `models.json` before this process performs a save.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum WorkBuddyConfigFormat {
    LegacyArray,
    ObjectRoot,
    Missing,
}

/// Minimal, non-sensitive summary of the current WorkBuddy configuration.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkBuddyStatus {
    /// A stable display value, deliberately not an absolute user path.
    pub path: String,
    pub exists: bool,
    /// Count of unique, trimmed IDs in their first-occurrence order.
    pub model_count: usize,
    /// Opaque process-local revision of the complete file bytes.
    pub revision: Option<String>,
    pub backup_exists: bool,
    pub format: WorkBuddyConfigFormat,
}

/// The intentionally narrow projection used by the existing-models card.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkBuddyModelIdsResult {
    pub ids: Vec<String>,
    pub revision: Option<String>,
}

/// Input for a constrained WorkBuddy `GET <base>/models` request.
#[derive(Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FetchWorkBuddyModelsRequest {
    pub base_url: String,
    pub api_key: String,
    pub allow_no_api_key: bool,
}

/// A bounded, ordered list of fetched model IDs.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FetchWorkBuddyModelsResult {
    pub models: Vec<String>,
    pub truncated: bool,
}

/// Input for the revision-checked WorkBuddy save transaction.
///
/// The service owns trimming, ordering, de-duplication, URL normalization,
/// revision validation, and all on-disk changes. `overwrite_token` is an
/// opaque, short-lived, one-time capability issued only after a preflight has
/// found one or more existing target IDs.
#[derive(Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SaveWorkBuddyModelsRequest {
    pub base_url: String,
    pub api_key: String,
    pub allow_no_api_key: bool,
    #[serde(default)]
    pub selected_model_ids: Vec<String>,
    #[serde(default)]
    pub manual_model_ids: Vec<String>,
    #[serde(default)]
    pub clear_existing_api_keys: bool,
    pub expected_revision: Option<String>,
    #[serde(default)]
    pub overwrite_token: Option<String>,
}

impl fmt::Debug for FetchWorkBuddyModelsRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("FetchWorkBuddyModelsRequest")
            .field("base_url", &"[REDACTED]")
            .field("api_key", &"[REDACTED]")
            .field("allow_no_api_key", &self.allow_no_api_key)
            .finish()
    }
}

impl fmt::Debug for SaveWorkBuddyModelsRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SaveWorkBuddyModelsRequest")
            .field("base_url", &"[REDACTED]")
            .field("api_key", &"[REDACTED]")
            .field("allow_no_api_key", &self.allow_no_api_key)
            .field("selected_model_id_count", &self.selected_model_ids.len())
            .field("manual_model_id_count", &self.manual_model_ids.len())
            .field("clear_existing_api_keys", &self.clear_existing_api_keys)
            .field("expected_revision", &"[REDACTED]")
            .field("overwrite_token", &"[REDACTED]")
            .finish()
    }
}

/// Result of the save preflight or committed transaction.
///
/// Existing targets are not an error: the renderer receives one aggregate
/// confirmation requirement and must resubmit the *same* request with the
/// opaque token. A stale revision is likewise a normal user-resolvable state,
/// not an unstructured transport failure.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "state")]
pub enum SaveWorkBuddyModelsOutcome {
    #[serde(rename = "saved")]
    Saved {
        revision: String,
        #[serde(rename = "modelCount")]
        model_count: usize,
        #[serde(rename = "createdEntries")]
        created_entries: usize,
        #[serde(rename = "updatedEntries")]
        updated_entries: usize,
    },
    #[serde(rename = "overwrite_confirmation_required")]
    OverwriteConfirmationRequired {
        token: String,
        #[serde(rename = "existingIds")]
        existing_ids: Vec<String>,
    },
    #[serde(rename = "concurrent_modification")]
    ConcurrentModification,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_outcome_uses_the_documented_discriminated_union_shape() {
        let outcome = SaveWorkBuddyModelsOutcome::OverwriteConfirmationRequired {
            token: "opaque-token".to_string(),
            existing_ids: vec!["model-a".to_string()],
        };

        assert_eq!(
            serde_json::to_value(outcome).unwrap(),
            serde_json::json!({
                "state": "overwrite_confirmation_required",
                "token": "opaque-token",
                "existingIds": ["model-a"]
            })
        );
    }

    #[test]
    fn request_debug_output_redacts_credentials_urls_and_capabilities() {
        let request = SaveWorkBuddyModelsRequest {
            base_url: "https://example.test/?token=not-for-logs".to_string(),
            api_key: "TEST-SECRET-WORKBUDDY-KEY".to_string(),
            allow_no_api_key: false,
            selected_model_ids: vec!["model-a".to_string()],
            manual_model_ids: Vec::new(),
            clear_existing_api_keys: false,
            expected_revision: Some("opaque-revision".to_string()),
            overwrite_token: Some("opaque-token".to_string()),
        };

        let debug = format!("{request:?}");
        for secret in [
            "TEST-SECRET-WORKBUDDY-KEY",
            "not-for-logs",
            "opaque-revision",
            "opaque-token",
        ] {
            assert!(!debug.contains(secret));
        }
    }
}
