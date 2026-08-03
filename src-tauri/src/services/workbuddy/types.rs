//! Stable WorkBuddy IPC data-transfer objects.
//!
//! WorkBuddy intentionally remains outside the provider/AppType domain. These
//! types are owned by the dedicated WorkBuddy service and contain only the
//! fields needed to configure `~/.workbuddy/models.json` safely.

use serde::{Deserialize, Serialize};

/// A duplicate target ID discovered in the on-disk configuration.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateModelId {
    pub id: String,
    pub count: usize,
}

/// The policy used after the user has reviewed an existing duplicate-ID
/// conflict. The default never overwrites duplicate target entries.
#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DuplicatePolicy {
    #[default]
    Reject,
    UpdateAll,
}

/// Minimal, non-sensitive summary of the current WorkBuddy configuration.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkBuddyStatus {
    pub path: String,
    pub exists: bool,
    pub model_count: usize,
    pub revision: Option<String>,
    pub backup_exists: bool,
}

/// Input for a constrained WorkBuddy `GET <base>/models` request.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
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

/// Input for the WorkBuddy model upsert transaction.
///
/// The caller submits only selected automatic IDs and manual IDs. The service
/// owns trimming, ordering, de-duplication, URL normalization, revision
/// validation, and all on-disk changes.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
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
    pub duplicate_policy: DuplicatePolicy,
}

/// Result of a successful WorkBuddy upsert transaction.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SaveWorkBuddyModelsResult {
    pub revision: String,
    pub model_count: usize,
    pub created_entries: usize,
    pub updated_entries: usize,
    pub duplicate_ids: Vec<DuplicateModelId>,
}
