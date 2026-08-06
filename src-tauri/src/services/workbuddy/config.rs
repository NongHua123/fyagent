//! Transactional WorkBuddy `models.json` storage.
//!
//! The service owns the current-user path, strict input validation, opaque
//! revisions, short-lived overwrite capabilities, preservation of unknown JSON
//! fields, fixed backup behavior, and a replacement primitive that never
//! deletes an existing credential file before replacement.

use std::{
    collections::{HashMap, HashSet},
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::{Mutex as StdMutex, OnceLock},
    time::{Duration, Instant},
};

#[cfg(unix)]
use std::fs::File;

use hmac::{Hmac, Mac};
use serde_json::{Map, Value};
use sha2::Sha256;
use tokio::sync::Mutex;
use uuid::Uuid;

use super::{
    document::WorkBuddyDocument,
    error::{WorkBuddyError, WorkBuddyErrorCode},
    types::{
        SaveWorkBuddyModelsOutcome, SaveWorkBuddyModelsRequest, WorkBuddyModelIdsResult,
        WorkBuddyStatus,
    },
    url::{normalize_workbuddy_base_url, NormalizedWorkBuddyUrl},
};

const MODELS_FILE_NAME: &str = "models.json";
const BACKUP_FILE_NAME: &str = "models.json.backup";
const DISPLAY_PATH: &str = ".workbuddy/models.json";
const OVERWRITE_TOKEN_TTL: Duration = Duration::from_secs(3 * 60);
type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone)]
pub(crate) struct WorkBuddyPaths {
    pub(crate) directory: PathBuf,
    pub(crate) models: PathBuf,
    pub(crate) backup: PathBuf,
}

impl WorkBuddyPaths {
    pub(crate) fn from_home(home: &Path) -> Self {
        let directory = home.join(".workbuddy");
        Self {
            models: directory.join(MODELS_FILE_NAME),
            backup: directory.join(BACKUP_FILE_NAME),
            directory,
        }
    }
}

#[derive(Debug)]
struct LoadedConfig {
    exists: bool,
    original_bytes: Vec<u8>,
    revision: Option<String>,
    document: WorkBuddyDocument,
}

/// Server-side state for an aggregate overwrite confirmation.
///
/// Only hashes, a revision, and an expiry live here. The opaque token does not
/// encode a path, model ID, credential, or request value.
#[derive(Debug)]
struct PendingOverwrite {
    request_digest: [u8; 32],
    expected_revision: Option<String>,
    expires_at: Instant,
}

pub(crate) async fn get_workbuddy_status() -> Result<WorkBuddyStatus, WorkBuddyError> {
    let paths = current_paths();
    tokio::task::spawn_blocking(move || get_workbuddy_status_at(&paths))
        .await
        .map_err(|_| WorkBuddyError::new(WorkBuddyErrorCode::InternalError))?
}

pub(crate) async fn get_workbuddy_model_ids() -> Result<WorkBuddyModelIdsResult, WorkBuddyError> {
    let paths = current_paths();
    tokio::task::spawn_blocking(move || get_workbuddy_model_ids_at(&paths))
        .await
        .map_err(|_| WorkBuddyError::new(WorkBuddyErrorCode::InternalError))?
}

pub(crate) async fn save_workbuddy_models(
    request: SaveWorkBuddyModelsRequest,
) -> Result<SaveWorkBuddyModelsOutcome, WorkBuddyError> {
    save_workbuddy_models_at(current_paths(), request).await
}

async fn save_workbuddy_models_at(
    paths: WorkBuddyPaths,
    request: SaveWorkBuddyModelsRequest,
) -> Result<SaveWorkBuddyModelsOutcome, WorkBuddyError> {
    let _guard = write_lock().lock().await;
    tokio::task::spawn_blocking(move || save_workbuddy_models_at_locked(&paths, &request))
        .await
        .map_err(|_| WorkBuddyError::new(WorkBuddyErrorCode::InternalError))?
}

fn current_paths() -> WorkBuddyPaths {
    WorkBuddyPaths::from_home(&crate::config::get_home_dir())
}

fn write_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn pending_overwrites() -> &'static StdMutex<HashMap<String, PendingOverwrite>> {
    static PENDING: OnceLock<StdMutex<HashMap<String, PendingOverwrite>>> = OnceLock::new();
    PENDING.get_or_init(|| StdMutex::new(HashMap::new()))
}

pub(crate) fn get_workbuddy_status_at(
    paths: &WorkBuddyPaths,
) -> Result<WorkBuddyStatus, WorkBuddyError> {
    let loaded = load_config(&paths.models)?;
    Ok(WorkBuddyStatus {
        path: DISPLAY_PATH.to_string(),
        exists: loaded.exists,
        model_count: loaded.document.unique_model_ids().len(),
        revision: loaded.revision,
        backup_exists: paths.backup.exists(),
        format: loaded.document.format(),
    })
}

pub(crate) fn get_workbuddy_model_ids_at(
    paths: &WorkBuddyPaths,
) -> Result<WorkBuddyModelIdsResult, WorkBuddyError> {
    let loaded = load_config(&paths.models)?;
    Ok(WorkBuddyModelIdsResult {
        ids: loaded.document.unique_model_ids(),
        revision: loaded.revision,
    })
}

pub(crate) fn save_workbuddy_models_at_locked(
    paths: &WorkBuddyPaths,
    request: &SaveWorkBuddyModelsRequest,
) -> Result<SaveWorkBuddyModelsOutcome, WorkBuddyError> {
    let normalized_url = normalize_workbuddy_base_url(&request.base_url)?;
    if request.api_key.trim().is_empty() && !request.allow_no_api_key {
        return Err(WorkBuddyError::new(WorkBuddyErrorCode::ApiKeyRequired));
    }
    let target_ids = normalized_target_ids(request);
    if target_ids.is_empty() {
        return Err(WorkBuddyError::new(
            WorkBuddyErrorCode::ConfigNoTargetModels,
        ));
    }

    // Consume a confirmation capability before the latest file reread. A
    // stale or malformed follow-up must not leave a reusable token behind.
    // The process lock serializes FyAgent writes; the revision checks below
    // detect user/editor changes. We intentionally never repair bad JSON.
    let pending = request
        .overwrite_token
        .as_deref()
        .map(|token| consume_overwrite_token(token, request, &normalized_url, &target_ids))
        .transpose()?;

    let mut loaded = load_config(&paths.models)?;
    if request.expected_revision != loaded.revision {
        return Ok(SaveWorkBuddyModelsOutcome::ConcurrentModification);
    }

    let existing_ids = loaded.document.existing_target_ids(&target_ids);
    if let Some(pending) = pending {
        if pending.expected_revision != loaded.revision {
            return Ok(SaveWorkBuddyModelsOutcome::ConcurrentModification);
        }
    } else if !existing_ids.is_empty() {
        let token = issue_overwrite_token(request, &normalized_url, &target_ids);
        return Ok(SaveWorkBuddyModelsOutcome::OverwriteConfirmationRequired {
            token,
            existing_ids,
        });
    }

    let normalized_base_url = normalized_url.base_url.to_string();
    let mut created_entries = 0usize;
    let mut updated_entries = 0usize;

    for target_id in &target_ids {
        let mut matched_existing = false;
        for entry in loaded.document.models_mut() {
            let model = entry
                .as_object_mut()
                .expect("document validation guarantees model-object entries");
            let matches_target = model
                .get("id")
                .and_then(Value::as_str)
                .is_some_and(|id| id.trim() == target_id);
            if matches_target {
                patch_existing_connection_fields(model, &normalized_base_url, request);
                updated_entries += 1;
                matched_existing = true;
            }
        }
        if !matched_existing {
            loaded
                .document
                .models_mut()
                .push(Value::Object(new_managed_model(
                    target_id,
                    &normalized_base_url,
                    request,
                )));
            created_entries += 1;
        }
    }

    // This runs before backups and the primary write. Therefore a malformed
    // `availableModels` field cannot produce a partial configuration update.
    loaded.document.update_available_models(&target_ids)?;
    let model_count = loaded.document.unique_model_ids().len();
    let serialized = loaded.document.serialize()?;

    if loaded.exists {
        write_credential_file_atomically(&paths.backup, &loaded.original_bytes)
            .map_err(|_| WorkBuddyError::new(WorkBuddyErrorCode::ConfigBackupFailed))?;
    } else if fs::create_dir_all(&paths.directory).is_err() {
        return Err(WorkBuddyError::new(WorkBuddyErrorCode::ConfigWriteFailed));
    }

    write_credential_file_atomically(&paths.models, &serialized)
        .map_err(|_| WorkBuddyError::new(WorkBuddyErrorCode::ConfigWriteFailed))?;

    Ok(SaveWorkBuddyModelsOutcome::Saved {
        revision: revision_for(&serialized),
        model_count,
        created_entries,
        updated_entries,
    })
}

fn load_config(path: &Path) -> Result<LoadedConfig, WorkBuddyError> {
    let original_bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Ok(LoadedConfig {
                exists: false,
                original_bytes: Vec::new(),
                revision: None,
                document: WorkBuddyDocument::missing(),
            });
        }
        Err(_) => return Err(WorkBuddyError::new(WorkBuddyErrorCode::ConfigReadFailed)),
    };

    let root: Value = serde_json::from_slice(&original_bytes)
        .map_err(|_| WorkBuddyError::new(WorkBuddyErrorCode::ConfigInvalidJson))?;
    let document = WorkBuddyDocument::parse(root)?;
    Ok(LoadedConfig {
        exists: true,
        revision: Some(revision_for(&original_bytes)),
        original_bytes,
        document,
    })
}

fn normalized_target_ids(request: &SaveWorkBuddyModelsRequest) -> Vec<String> {
    let mut target_ids = Vec::new();
    let mut seen = HashSet::new();
    for id in request
        .selected_model_ids
        .iter()
        .chain(request.manual_model_ids.iter())
    {
        let id = id.trim();
        if !id.is_empty() && seen.insert(id.to_string()) {
            target_ids.push(id.to_string());
        }
    }
    target_ids
}

fn new_managed_model(
    id: &str,
    normalized_base_url: &str,
    request: &SaveWorkBuddyModelsRequest,
) -> Map<String, Value> {
    let mut model = Map::new();
    model.insert("id".to_string(), Value::String(id.to_string()));
    model.insert("name".to_string(), Value::String(id.to_string()));
    model.insert("vendor".to_string(), Value::String("Custom".to_string()));
    model.insert(
        "url".to_string(),
        Value::String(normalized_base_url.to_string()),
    );
    model.insert("apiKey".to_string(), Value::String(request.api_key.clone()));
    model.insert("supportsToolCall".to_string(), Value::Bool(true));
    model.insert("supportsImages".to_string(), Value::Bool(true));
    model.insert("supportsReasoning".to_string(), Value::Bool(true));
    model.insert("useCustomProtocol".to_string(), Value::Bool(false));
    model.insert(
        "reasoning".to_string(),
        serde_json::json!({
            "defaultEffort": "max",
            "supportedEfforts": ["low", "medium", "high", "xhigh", "max"],
            "canDisableThinking": false,
        }),
    );
    model
}

/// Patch the two documented connection fields on an existing entry.
///
/// Importantly, this does not normalize IDs, rebuild the managed template, or
/// remove `onlyReasoning`: every other field belongs to the existing document.
fn patch_existing_connection_fields(
    model: &mut Map<String, Value>,
    normalized_base_url: &str,
    request: &SaveWorkBuddyModelsRequest,
) {
    model.insert(
        "url".to_string(),
        Value::String(normalized_base_url.to_string()),
    );
    if !request.api_key.trim().is_empty() {
        model.insert("apiKey".to_string(), Value::String(request.api_key.clone()));
    } else if request.clear_existing_api_keys {
        model.insert("apiKey".to_string(), Value::String(String::new()));
    }
}

fn issue_overwrite_token(
    request: &SaveWorkBuddyModelsRequest,
    normalized_url: &NormalizedWorkBuddyUrl,
    target_ids: &[String],
) -> String {
    let token = new_opaque_capability_token();
    let pending = PendingOverwrite {
        request_digest: request_digest(request, normalized_url, target_ids),
        expected_revision: request.expected_revision.clone(),
        expires_at: Instant::now() + OVERWRITE_TOKEN_TTL,
    };
    let mut pending_overwrites = lock_pending_overwrites();
    let now = Instant::now();
    pending_overwrites.retain(|_, pending| pending.expires_at > now);
    pending_overwrites.insert(token.clone(), pending);
    token
}

/// Two independent UUID v4 values retain 244 random bits after their fixed
/// version/variant bits.  This clears the 128-bit entropy floor while keeping
/// the capability an opaque, URL-safe token without a new dependency.
fn new_opaque_capability_token() -> String {
    format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple())
}

fn consume_overwrite_token(
    token: &str,
    request: &SaveWorkBuddyModelsRequest,
    normalized_url: &NormalizedWorkBuddyUrl,
    target_ids: &[String],
) -> Result<PendingOverwrite, WorkBuddyError> {
    // Remove first so malformed, mismatched, expired, and successful attempts
    // all consume a token exactly once.
    let pending = lock_pending_overwrites()
        .remove(token)
        .ok_or_else(|| WorkBuddyError::new(WorkBuddyErrorCode::OverwriteTokenInvalid))?;
    if pending.expires_at <= Instant::now() {
        return Err(WorkBuddyError::new(
            WorkBuddyErrorCode::OverwriteTokenExpired,
        ));
    }

    let request_digest = request_digest(request, normalized_url, target_ids);
    if !constant_time_equals(&pending.request_digest, &request_digest) {
        return Err(WorkBuddyError::new(
            WorkBuddyErrorCode::OverwriteTokenInvalid,
        ));
    }
    Ok(pending)
}

fn lock_pending_overwrites() -> std::sync::MutexGuard<'static, HashMap<String, PendingOverwrite>> {
    pending_overwrites()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Hash the canonical request with a process-local key. The API key enters the
/// digest only through a second process-local MAC, so pending state cannot
/// expose it or enable an offline equality/dictionary check after serialization.
fn request_digest(
    request: &SaveWorkBuddyModelsRequest,
    normalized_url: &NormalizedWorkBuddyUrl,
    target_ids: &[String],
) -> [u8; 32] {
    let mut mac = HmacSha256::new_from_slice(overwrite_mac_key())
        .expect("the fixed-size overwrite MAC key is always valid");
    mac.update(b"fyagent-workbuddy-overwrite-v1");
    update_length_prefixed(&mut mac, normalized_url.base_url.as_str().as_bytes());
    update_bool(&mut mac, request.allow_no_api_key);
    update_bool(&mut mac, request.clear_existing_api_keys);
    update_optional_string(&mut mac, request.expected_revision.as_deref());
    for target_id in target_ids {
        update_length_prefixed(&mut mac, target_id.as_bytes());
    }
    mac.update(&(target_ids.len() as u64).to_be_bytes());
    let api_key_digest = mac_bytes(api_key_mac_key(), request.api_key.as_bytes());
    update_length_prefixed(&mut mac, &api_key_digest);
    mac_bytes_from_mac(mac)
}

fn update_length_prefixed(mac: &mut HmacSha256, bytes: &[u8]) {
    mac.update(&(bytes.len() as u64).to_be_bytes());
    mac.update(bytes);
}

fn update_optional_string(mac: &mut HmacSha256, value: Option<&str>) {
    match value {
        Some(value) => {
            mac.update(&[1]);
            update_length_prefixed(mac, value.as_bytes());
        }
        None => mac.update(&[0]),
    }
}

fn update_bool(mac: &mut HmacSha256, value: bool) {
    mac.update(&[u8::from(value)]);
}

fn mac_bytes(key: &[u8; 32], bytes: &[u8]) -> [u8; 32] {
    let mut mac = HmacSha256::new_from_slice(key).expect("the fixed-size MAC key is always valid");
    mac.update(bytes);
    mac_bytes_from_mac(mac)
}

fn mac_bytes_from_mac(mac: HmacSha256) -> [u8; 32] {
    let bytes = mac.finalize().into_bytes();
    let mut output = [0u8; 32];
    output.copy_from_slice(&bytes);
    output
}

fn constant_time_equals(left: &[u8; 32], right: &[u8; 32]) -> bool {
    left.iter()
        .zip(right)
        .fold(0u8, |difference, (left, right)| difference | (left ^ right))
        == 0
}

fn revision_for(bytes: &[u8]) -> String {
    mac_bytes(revision_mac_key(), bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn revision_mac_key() -> &'static [u8; 32] {
    static KEY: OnceLock<[u8; 32]> = OnceLock::new();
    KEY.get_or_init(random_mac_key)
}

fn overwrite_mac_key() -> &'static [u8; 32] {
    static KEY: OnceLock<[u8; 32]> = OnceLock::new();
    KEY.get_or_init(random_mac_key)
}

fn api_key_mac_key() -> &'static [u8; 32] {
    static KEY: OnceLock<[u8; 32]> = OnceLock::new();
    KEY.get_or_init(random_mac_key)
}

fn random_mac_key() -> [u8; 32] {
    let mut key = [0u8; 32];
    key[..16].copy_from_slice(Uuid::new_v4().as_bytes());
    key[16..].copy_from_slice(Uuid::new_v4().as_bytes());
    key
}

fn write_credential_file_atomically(path: &Path, data: &[u8]) -> io::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "missing parent directory"))?;
    fs::create_dir_all(parent)?;

    let temp = create_temp_file(parent, path.file_name().unwrap_or_default(), data)?;
    let result = replace_file(&temp, path).and_then(|_| sync_parent_directory(parent));
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}

fn create_temp_file(
    parent: &Path,
    file_name: &std::ffi::OsStr,
    data: &[u8],
) -> io::Result<PathBuf> {
    let file_name = file_name.to_string_lossy();
    for _ in 0..5 {
        let temp = parent.join(format!(".{file_name}.tmp.{}", Uuid::new_v4()));
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }

        let mut file = match options.open(&temp) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        };
        let write_result = (|| {
            file.write_all(data)?;
            file.flush()?;
            file.sync_all()?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(&temp, fs::Permissions::from_mode(0o600))?;
            }
            Ok(())
        })();
        if let Err(error) = write_result {
            let _ = fs::remove_file(&temp);
            return Err(error);
        }
        return Ok(temp);
    }

    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "could not create a unique temporary file",
    ))
}

#[cfg(windows)]
fn replace_file(temp: &Path, target: &Path) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows::{
        core::PCWSTR,
        Win32::Storage::FileSystem::{
            MoveFileExW, ReplaceFileW, MOVEFILE_WRITE_THROUGH, REPLACEFILE_WRITE_THROUGH,
        },
    };

    let temp_wide = temp
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let target_wide = target
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    // Existing credential files use ReplaceFileW so Windows preserves the
    // destination's ACL/metadata in the platform replacement operation. The
    // first creation has no destination to replace and uses same-directory
    // MoveFileExW with write-through. Neither path deletes the target first.
    unsafe {
        if target.exists() {
            ReplaceFileW(
                PCWSTR(target_wide.as_ptr()),
                PCWSTR(temp_wide.as_ptr()),
                PCWSTR::null(),
                REPLACEFILE_WRITE_THROUGH,
                None,
                None,
            )
        } else {
            MoveFileExW(
                PCWSTR(temp_wide.as_ptr()),
                PCWSTR(target_wide.as_ptr()),
                MOVEFILE_WRITE_THROUGH,
            )
        }
        .map_err(|_| io::Error::last_os_error())
    }
}

#[cfg(not(windows))]
fn replace_file(temp: &Path, target: &Path) -> io::Result<()> {
    fs::rename(temp, target)
}

#[cfg(unix)]
fn sync_parent_directory(parent: &Path) -> io::Result<()> {
    File::open(parent)?.sync_all()
}

#[cfg(not(unix))]
fn sync_parent_directory(_parent: &Path) -> io::Result<()> {
    Ok(())
}

#[cfg(test)]
fn expire_overwrite_token_for_test(token: &str) {
    if let Some(pending) = lock_pending_overwrites().get_mut(token) {
        pending.expires_at = Instant::now() - Duration::from_secs(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::workbuddy::types::{SaveWorkBuddyModelsOutcome, WorkBuddyConfigFormat};
    use sha2::Digest;

    #[test]
    fn overwrite_capabilities_exceed_the_required_random_entropy_floor() {
        let first = new_opaque_capability_token();
        let second = new_opaque_capability_token();

        assert_eq!(first.len(), 64);
        assert_eq!(second.len(), 64);
        assert_ne!(first, second);
    }

    fn paths(temp: &tempfile::TempDir) -> WorkBuddyPaths {
        WorkBuddyPaths::from_home(temp.path())
    }

    fn request(expected_revision: Option<String>) -> SaveWorkBuddyModelsRequest {
        SaveWorkBuddyModelsRequest {
            base_url: "https://api.example.test".to_string(),
            api_key: "TEST-SECRET-WORKBUDDY-KEY".to_string(),
            allow_no_api_key: false,
            selected_model_ids: vec!["model-a".to_string()],
            manual_model_ids: Vec::new(),
            clear_existing_api_keys: false,
            expected_revision,
            overwrite_token: None,
        }
    }

    fn write_models(paths: &WorkBuddyPaths, contents: &str) {
        fs::create_dir_all(&paths.directory).unwrap();
        fs::write(&paths.models, contents.as_bytes()).unwrap();
    }

    fn read_json(paths: &WorkBuddyPaths) -> Value {
        serde_json::from_slice(&fs::read(&paths.models).unwrap()).unwrap()
    }

    fn saved(outcome: SaveWorkBuddyModelsOutcome) -> (String, usize, usize, usize) {
        match outcome {
            SaveWorkBuddyModelsOutcome::Saved {
                revision,
                model_count,
                created_entries,
                updated_entries,
            } => (revision, model_count, created_entries, updated_entries),
            other => panic!("expected a saved outcome, got {other:?}"),
        }
    }

    fn overwrite(outcome: SaveWorkBuddyModelsOutcome) -> (String, Vec<String>) {
        match outcome {
            SaveWorkBuddyModelsOutcome::OverwriteConfirmationRequired {
                token,
                existing_ids,
            } => (token, existing_ids),
            other => panic!("expected an overwrite confirmation, got {other:?}"),
        }
    }

    #[test]
    fn first_save_creates_an_object_root_without_a_backup_and_with_managed_fields() {
        let temp = tempfile::tempdir().unwrap();
        let paths = paths(&temp);
        let (_, model_count, created_entries, updated_entries) =
            saved(save_workbuddy_models_at_locked(&paths, &request(None)).unwrap());

        assert_eq!((model_count, created_entries, updated_entries), (1, 1, 0));
        assert!(
            !paths.backup.exists(),
            "first creation must not create a backup"
        );
        let root = read_json(&paths);
        let model = root["models"][0].as_object().unwrap();
        assert_eq!(model.get("id"), Some(&Value::String("model-a".to_string())));
        assert_eq!(
            model.get("apiKey"),
            Some(&Value::String("TEST-SECRET-WORKBUDDY-KEY".to_string()))
        );
        assert_eq!(
            model.get("url"),
            Some(&Value::String("https://api.example.test/v1".to_string()))
        );
        assert_eq!(model.get("supportsToolCall"), Some(&Value::Bool(true)));
    }

    #[test]
    fn array_root_round_trips_unknown_fields_and_stays_an_array() {
        let temp = tempfile::tempdir().unwrap();
        let paths = paths(&temp);
        write_models(&paths, r#"[{"id":"old","unknown":{"kept":true}}]"#);
        let revision = get_workbuddy_status_at(&paths).unwrap().revision;
        let mut request = request(revision);
        request.selected_model_ids = vec!["new".to_string()];

        saved(save_workbuddy_models_at_locked(&paths, &request).unwrap());
        let root = read_json(&paths);
        let models = root.as_array().expect("legacy root must remain an array");
        assert_eq!(models[0]["unknown"], serde_json::json!({"kept": true}));
        assert_eq!(models[1]["id"], "new");
    }

    #[test]
    fn object_root_missing_models_adds_only_models_and_preserves_top_level_order_and_values() {
        let temp = tempfile::tempdir().unwrap();
        let paths = paths(&temp);
        write_models(&paths, r#"{"theme":"dark","future":{"kept":true}}"#);
        let revision = get_workbuddy_status_at(&paths).unwrap().revision;

        saved(save_workbuddy_models_at_locked(&paths, &request(revision)).unwrap());
        let root = read_json(&paths);
        assert_eq!(root["theme"], "dark");
        assert_eq!(root["future"], serde_json::json!({"kept": true}));
        assert_eq!(root["models"][0]["id"], "model-a");
    }

    #[test]
    fn existing_object_and_model_key_order_survive_a_non_conflicting_append() {
        let temp = tempfile::tempdir().unwrap();
        let paths = paths(&temp);
        write_models(
            &paths,
            r#"{"first":1,"models":[{"id":"old","z":true,"a":{"kept":1}}],"last":2}"#,
        );
        let revision = get_workbuddy_status_at(&paths).unwrap().revision;
        let mut request = request(revision);
        request.selected_model_ids = vec!["new".to_string()];

        saved(save_workbuddy_models_at_locked(&paths, &request).unwrap());
        let serialized = String::from_utf8(fs::read(&paths.models).unwrap()).unwrap();
        let root_first = serialized.find("\"first\"").unwrap();
        let root_models = serialized.find("\"models\"").unwrap();
        let root_last = serialized.find("\"last\"").unwrap();
        let model_id = serialized.find("\"id\": \"old\"").unwrap();
        let model_z = serialized.find("\"z\": true").unwrap();
        let model_a = serialized.find("\"a\"").unwrap();
        assert!(root_first < root_models && root_models < root_last);
        assert!(model_id < model_z && model_z < model_a);
    }

    #[test]
    fn status_and_id_projection_are_deidentified_unique_and_consistent() {
        let temp = tempfile::tempdir().unwrap();
        let paths = paths(&temp);
        write_models(
            &paths,
            r#"{"models":[{"id":" model-a ","apiKey":"first"},{"id":"model-a","apiKey":"second"},{"id":"Model-A"}]}"#,
        );

        let status = get_workbuddy_status_at(&paths).unwrap();
        let ids = get_workbuddy_model_ids_at(&paths).unwrap();
        let serialized_status = serde_json::to_string(&status).unwrap();
        assert_eq!(status.path, DISPLAY_PATH);
        assert_eq!(status.format, WorkBuddyConfigFormat::ObjectRoot);
        assert_eq!(status.model_count, 2);
        assert_eq!(ids.ids, ["model-a", "Model-A"]);
        assert_eq!(ids.revision, status.revision);
        assert!(!serialized_status.contains("first"));
        assert!(!serialized_status.contains("model-a"));
    }

    #[test]
    fn invalid_documents_fail_without_backup_or_primary_repair() {
        let temp = tempfile::tempdir().unwrap();
        let paths = paths(&temp);
        for (contents, expected_code) in [
            (r#""root""#, WorkBuddyErrorCode::ConfigRootUnsupported),
            (r#"{"models":{}}"#, WorkBuddyErrorCode::ConfigModelsNotArray),
            (
                r#"{"models":[{"id":"ok"},7]}"#,
                WorkBuddyErrorCode::ConfigInvalidEntry,
            ),
            (r#"[{"id":"  "}]"#, WorkBuddyErrorCode::ConfigInvalidEntry),
        ] {
            write_models(&paths, contents);
            let before = fs::read(&paths.models).unwrap();
            let error = save_workbuddy_models_at_locked(&paths, &request(None)).unwrap_err();
            assert_eq!(error.code(), expected_code);
            assert_eq!(fs::read(&paths.models).unwrap(), before);
            assert!(!paths.backup.exists());
            let _ = fs::remove_file(&paths.models);
        }
    }

    #[test]
    fn existing_ids_require_one_aggregate_confirmation_and_do_not_write_at_preflight() {
        let temp = tempfile::tempdir().unwrap();
        let paths = paths(&temp);
        write_models(&paths, r#"[{"id":"a"},{"id":"b"}]"#);
        let before = fs::read(&paths.models).unwrap();
        let revision = get_workbuddy_status_at(&paths).unwrap().revision;
        let mut request = request(revision);
        request.selected_model_ids = vec!["b".to_string(), "c".to_string(), "a".to_string()];

        let (_, existing_ids) =
            overwrite(save_workbuddy_models_at_locked(&paths, &request).unwrap());
        assert_eq!(existing_ids, ["b", "a"]);
        assert_eq!(fs::read(&paths.models).unwrap(), before);
        assert!(!paths.backup.exists());
    }

    #[test]
    fn confirmed_mixed_save_updates_all_historical_matches_and_preserves_every_other_field() {
        let temp = tempfile::tempdir().unwrap();
        let paths = paths(&temp);
        write_models(
            &paths,
            r#"{"models":[
                {"id":" model-a ","url":"old-1","apiKey":"first","name":"kept-1","onlyReasoning":true,"reasoning":{"future":1},"future":"one"},
                {"id":"other","future":"untouched"},
                {"id":"model-a","url":"old-2","apiKey":"second","vendor":"kept-2","maxInputTokens":99,"future":"two"}
            ]}"#,
        );
        let revision = get_workbuddy_status_at(&paths).unwrap().revision;
        let mut request = request(revision);
        request.selected_model_ids = vec!["model-a".to_string(), "new".to_string()];

        let (token, existing_ids) =
            overwrite(save_workbuddy_models_at_locked(&paths, &request).unwrap());
        assert_eq!(existing_ids, ["model-a"]);
        request.overwrite_token = Some(token);
        let (_, _, created_entries, updated_entries) =
            saved(save_workbuddy_models_at_locked(&paths, &request).unwrap());
        assert_eq!((created_entries, updated_entries), (1, 2));

        let root = read_json(&paths);
        let models = root["models"].as_array().unwrap();
        assert_eq!(models.len(), 4);
        assert_eq!(models[0]["id"], " model-a ");
        assert_eq!(models[0]["url"], "https://api.example.test/v1");
        assert_eq!(models[0]["apiKey"], "TEST-SECRET-WORKBUDDY-KEY");
        assert_eq!(models[0]["name"], "kept-1");
        assert_eq!(models[0]["onlyReasoning"], true);
        assert_eq!(models[0]["reasoning"], serde_json::json!({"future": 1}));
        assert_eq!(
            models[1],
            serde_json::json!({"id":"other","future":"untouched"})
        );
        assert_eq!(models[2]["vendor"], "kept-2");
        assert_eq!(models[2]["maxInputTokens"], 99);
        assert_eq!(models[3]["id"], "new");
    }

    #[test]
    fn empty_key_preserves_each_existing_key_unless_explicitly_cleared() {
        let temp = tempfile::tempdir().unwrap();
        let paths = paths(&temp);
        write_models(
            &paths,
            r#"[{"id":"model-a","apiKey":"first-key"},{"id":"model-a","apiKey":"second-key"}]"#,
        );
        let revision = get_workbuddy_status_at(&paths).unwrap().revision;
        let mut request = request(revision);
        request.api_key.clear();
        request.allow_no_api_key = true;
        let (token, _) = overwrite(save_workbuddy_models_at_locked(&paths, &request).unwrap());
        request.overwrite_token = Some(token);
        let (revision, ..) = saved(save_workbuddy_models_at_locked(&paths, &request).unwrap());
        let kept = read_json(&paths);
        assert_eq!(kept[0]["apiKey"], "first-key");
        assert_eq!(kept[1]["apiKey"], "second-key");

        request.expected_revision = Some(revision);
        request.clear_existing_api_keys = true;
        request.overwrite_token = None;
        let (token, _) = overwrite(save_workbuddy_models_at_locked(&paths, &request).unwrap());
        request.overwrite_token = Some(token);
        saved(save_workbuddy_models_at_locked(&paths, &request).unwrap());
        let cleared = read_json(&paths);
        assert_eq!(cleared[0]["apiKey"], "");
        assert_eq!(cleared[1]["apiKey"], "");
    }

    #[test]
    fn available_models_follows_the_three_object_root_branches_without_migration() {
        let temp = tempfile::tempdir().unwrap();
        let paths = paths(&temp);

        for (contents, expected) in [
            (r#"{"models":[]}"#, None),
            (
                r#"{"models":[],"availableModels":[]}"#,
                Some(serde_json::json!([])),
            ),
            (
                r#"{"models":[],"availableModels":["old","model-a"]}"#,
                Some(serde_json::json!(["old", "model-a"])),
            ),
        ] {
            write_models(&paths, contents);
            let revision = get_workbuddy_status_at(&paths).unwrap().revision;
            let mut request = request(revision);
            request.selected_model_ids = vec!["model-a".to_string(), "model-b".to_string()];
            saved(save_workbuddy_models_at_locked(&paths, &request).unwrap());
            let root = read_json(&paths);
            match expected {
                None => assert!(root.get("availableModels").is_none()),
                Some(Value::Array(items)) if items.is_empty() => {
                    assert_eq!(root["availableModels"], serde_json::json!([]));
                }
                Some(Value::Array(_)) => assert_eq!(
                    root["availableModels"],
                    serde_json::json!(["old", "model-a", "model-b"])
                ),
                Some(_) => unreachable!(),
            }
        }

        write_models(&paths, r#"[{"id":"old"}]"#);
        let revision = get_workbuddy_status_at(&paths).unwrap().revision;
        let mut request = request(revision);
        request.selected_model_ids = vec!["new".to_string()];
        saved(save_workbuddy_models_at_locked(&paths, &request).unwrap());
        assert!(read_json(&paths).get("availableModels").is_none());
    }

    #[test]
    fn invalid_available_models_aborts_without_backup_or_primary_write() {
        let temp = tempfile::tempdir().unwrap();
        let paths = paths(&temp);
        for contents in [
            r#"{"models":[],"availableModels":{}}"#,
            r#"{"models":[],"availableModels":["valid",2]}"#,
        ] {
            write_models(&paths, contents);
            let before = fs::read(&paths.models).unwrap();
            let revision = get_workbuddy_status_at(&paths).unwrap().revision;
            let error = save_workbuddy_models_at_locked(&paths, &request(revision)).unwrap_err();
            assert_eq!(error.code(), WorkBuddyErrorCode::ConfigInvalidEntry);
            assert_eq!(fs::read(&paths.models).unwrap(), before);
            assert!(!paths.backup.exists());
            let _ = fs::remove_file(&paths.models);
        }
    }

    #[test]
    fn overwrite_token_is_one_time_binds_the_full_normalized_request_and_detects_concurrency() {
        let temp = tempfile::tempdir().unwrap();
        let paths = paths(&temp);
        write_models(&paths, r#"[{"id":"model-a","apiKey":"old"}]"#);
        let revision = get_workbuddy_status_at(&paths).unwrap().revision;
        let request = request(revision);
        let before = fs::read(&paths.models).unwrap();

        let (token, _) = overwrite(save_workbuddy_models_at_locked(&paths, &request).unwrap());
        let mut key_changed = request.clone();
        key_changed.api_key = "another-secret".to_string();
        key_changed.overwrite_token = Some(token.clone());
        assert_eq!(
            save_workbuddy_models_at_locked(&paths, &key_changed)
                .unwrap_err()
                .code(),
            WorkBuddyErrorCode::OverwriteTokenInvalid
        );
        assert_eq!(fs::read(&paths.models).unwrap(), before);

        let mut consumed = request.clone();
        consumed.overwrite_token = Some(token);
        assert_eq!(
            save_workbuddy_models_at_locked(&paths, &consumed)
                .unwrap_err()
                .code(),
            WorkBuddyErrorCode::OverwriteTokenInvalid
        );

        let (token, _) = overwrite(save_workbuddy_models_at_locked(&paths, &request).unwrap());
        fs::write(
            &paths.models,
            r#"[{"id":"model-a","apiKey":"externally-rotated"}]"#,
        )
        .unwrap();
        let mut concurrent = request.clone();
        concurrent.overwrite_token = Some(token.clone());
        assert_eq!(
            save_workbuddy_models_at_locked(&paths, &concurrent).unwrap(),
            SaveWorkBuddyModelsOutcome::ConcurrentModification
        );
        assert_eq!(
            save_workbuddy_models_at_locked(&paths, &concurrent)
                .unwrap_err()
                .code(),
            WorkBuddyErrorCode::OverwriteTokenInvalid
        );
        assert!(!paths.backup.exists());
    }

    #[test]
    fn expired_overwrite_token_is_rejected_without_a_write() {
        let temp = tempfile::tempdir().unwrap();
        let paths = paths(&temp);
        write_models(&paths, r#"[{"id":"model-a"}]"#);
        let revision = get_workbuddy_status_at(&paths).unwrap().revision;
        let request = request(revision);
        let (token, _) = overwrite(save_workbuddy_models_at_locked(&paths, &request).unwrap());
        expire_overwrite_token_for_test(&token);
        let before = fs::read(&paths.models).unwrap();
        let mut confirmation = request;
        confirmation.overwrite_token = Some(token);

        assert_eq!(
            save_workbuddy_models_at_locked(&paths, &confirmation)
                .unwrap_err()
                .code(),
            WorkBuddyErrorCode::OverwriteTokenExpired
        );
        assert_eq!(fs::read(&paths.models).unwrap(), before);
    }

    #[test]
    fn revision_is_opaque_and_any_api_key_only_change_becomes_a_concurrent_outcome() {
        let temp = tempfile::tempdir().unwrap();
        let paths = paths(&temp);
        let original = br#"[{"id":"model-a","apiKey":"correct-horse-battery-staple"}]"#;
        write_models(&paths, std::str::from_utf8(original).unwrap());
        let stale_revision = get_workbuddy_status_at(&paths).unwrap().revision.unwrap();
        assert_ne!(stale_revision, format!("{:x}", Sha256::digest(original)));

        fs::write(
            &paths.models,
            r#"[{"id":"model-a","apiKey":"externally-rotated"}]"#,
        )
        .unwrap();
        let before = fs::read(&paths.models).unwrap();
        assert_eq!(
            save_workbuddy_models_at_locked(&paths, &request(Some(stale_revision))).unwrap(),
            SaveWorkBuddyModelsOutcome::ConcurrentModification
        );
        assert_eq!(fs::read(&paths.models).unwrap(), before);
        assert!(!paths.backup.exists());
    }

    #[test]
    fn backup_is_the_immediately_previous_file_and_failed_backup_keeps_primary() {
        let temp = tempfile::tempdir().unwrap();
        let paths = paths(&temp);
        write_models(&paths, r#"[{"id":"old"}]"#);
        let original = fs::read(&paths.models).unwrap();
        let revision = get_workbuddy_status_at(&paths).unwrap().revision;
        let mut first = request(revision);
        first.selected_model_ids = vec!["first".to_string()];
        saved(save_workbuddy_models_at_locked(&paths, &first).unwrap());
        assert_eq!(fs::read(&paths.backup).unwrap(), original);

        let before = fs::read(&paths.models).unwrap();
        let _ = fs::remove_file(&paths.backup);
        fs::create_dir(&paths.backup).unwrap();
        let revision = get_workbuddy_status_at(&paths).unwrap().revision;
        let error = save_workbuddy_models_at_locked(&paths, &request(revision)).unwrap_err();
        assert_eq!(error.code(), WorkBuddyErrorCode::ConfigBackupFailed);
        assert_eq!(fs::read(&paths.models).unwrap(), before);
    }

    #[tokio::test]
    async fn concurrent_saves_are_serialized_and_the_losing_revision_is_not_written() {
        let temp = tempfile::tempdir().unwrap();
        let paths = paths(&temp);
        let (first, second) = tokio::join!(
            save_workbuddy_models_at(paths.clone(), request(None)),
            save_workbuddy_models_at(paths.clone(), request(None)),
        );

        let outcomes = [first.unwrap(), second.unwrap()];
        assert!(outcomes
            .iter()
            .any(|outcome| matches!(outcome, SaveWorkBuddyModelsOutcome::Saved { .. })));
        assert!(outcomes
            .iter()
            .any(|outcome| matches!(outcome, SaveWorkBuddyModelsOutcome::ConcurrentModification)));
        assert_eq!(read_json(&paths)["models"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn error_and_success_dtos_never_serialize_the_request_api_key() {
        let temp = tempfile::tempdir().unwrap();
        let paths = paths(&temp);
        let request = request(None);
        let outcome = save_workbuddy_models_at_locked(&paths, &request).unwrap();
        let serialized_outcome = serde_json::to_string(&outcome).unwrap();
        assert!(!serialized_outcome.contains("TEST-SECRET-WORKBUDDY-KEY"));

        let error = WorkBuddyError::new(WorkBuddyErrorCode::ConfigNoTargetModels).to_dto();
        assert!(!serde_json::to_string(&error)
            .unwrap()
            .contains("TEST-SECRET-WORKBUDDY-KEY"));
    }

    #[test]
    fn failed_replacement_never_deletes_an_existing_target() {
        let temp = tempfile::tempdir().unwrap();
        let target = temp.path().join("models.json");
        fs::create_dir(&target).unwrap();

        assert!(write_credential_file_atomically(&target, b"new").is_err());
        assert!(
            target.is_dir(),
            "replacement failure must preserve the target"
        );
        assert!(
            fs::read_dir(temp.path()).unwrap().all(|entry| !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .contains(".tmp.")),
            "failed replacement must clean the same-directory temporary file"
        );
    }

    #[cfg(unix)]
    #[test]
    fn credential_files_are_created_with_user_only_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let temp = tempfile::tempdir().unwrap();
        let paths = paths(&temp);
        saved(save_workbuddy_models_at_locked(&paths, &request(None)).unwrap());
        assert_eq!(
            fs::metadata(&paths.models).unwrap().permissions().mode() & 0o077,
            0
        );

        let revision = get_workbuddy_status_at(&paths).unwrap().revision;
        let mut next = request(revision);
        next.selected_model_ids = vec!["model-b".to_string()];
        saved(save_workbuddy_models_at_locked(&paths, &next).unwrap());
        assert_eq!(
            fs::metadata(&paths.backup).unwrap().permissions().mode() & 0o077,
            0
        );
    }
}
