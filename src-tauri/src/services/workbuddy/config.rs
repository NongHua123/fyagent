//! Transactional WorkBuddy `models.json` storage.
//!
//! The service owns the current-user path, strict input validation, revision
//! checks, preservation of unknown JSON fields, fixed backup behavior, and a
//! stricter replacement primitive than the general application config writer.

use std::{
    collections::{HashMap, HashSet},
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::OnceLock,
};

#[cfg(unix)]
use std::fs::File;

use hmac::{Hmac, Mac};
use serde_json::{Map, Value};
use sha2::Sha256;
use tokio::sync::Mutex;
use uuid::Uuid;

use super::{
    error::{WorkBuddyError, WorkBuddyErrorCode},
    types::{
        DuplicateModelId, DuplicatePolicy, SaveWorkBuddyModelsRequest, SaveWorkBuddyModelsResult,
        WorkBuddyStatus,
    },
    url::normalize_workbuddy_base_url,
};

const MODELS_FILE_NAME: &str = "models.json";
const BACKUP_FILE_NAME: &str = "models.json.backup";
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
    models: Vec<Map<String, Value>>,
}

pub(crate) async fn get_workbuddy_status() -> Result<WorkBuddyStatus, WorkBuddyError> {
    let paths = current_paths();
    tokio::task::spawn_blocking(move || get_workbuddy_status_at(&paths))
        .await
        .map_err(|_| WorkBuddyError::new(WorkBuddyErrorCode::InternalError))?
}

pub(crate) async fn save_workbuddy_models(
    request: SaveWorkBuddyModelsRequest,
) -> Result<SaveWorkBuddyModelsResult, WorkBuddyError> {
    save_workbuddy_models_at(current_paths(), request).await
}

async fn save_workbuddy_models_at(
    paths: WorkBuddyPaths,
    request: SaveWorkBuddyModelsRequest,
) -> Result<SaveWorkBuddyModelsResult, WorkBuddyError> {
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

pub(crate) fn get_workbuddy_status_at(
    paths: &WorkBuddyPaths,
) -> Result<WorkBuddyStatus, WorkBuddyError> {
    let loaded = load_config(&paths.models)?;
    Ok(WorkBuddyStatus {
        path: paths.models.to_string_lossy().to_string(),
        exists: loaded.exists,
        model_count: loaded.models.len(),
        revision: loaded.revision,
        backup_exists: paths.backup.exists(),
    })
}

pub(crate) fn save_workbuddy_models_at_locked(
    paths: &WorkBuddyPaths,
    request: &SaveWorkBuddyModelsRequest,
) -> Result<SaveWorkBuddyModelsResult, WorkBuddyError> {
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

    let mut loaded = load_config(&paths.models)?;
    if request.expected_revision != loaded.revision {
        return Err(WorkBuddyError::new(
            WorkBuddyErrorCode::ConfigConcurrentModification,
        ));
    }

    let duplicate_ids = target_duplicate_ids(&loaded.models, &target_ids);
    if !duplicate_ids.is_empty() && request.duplicate_policy != DuplicatePolicy::UpdateAll {
        return Err(
            WorkBuddyError::new(WorkBuddyErrorCode::ConfigDuplicateTarget)
                .with_duplicate_ids(duplicate_ids),
        );
    }

    let mut created_entries = 0usize;
    let mut updated_entries = 0usize;
    let normalized_base_url = normalized_url.base_url.to_string();

    for target_id in &target_ids {
        let mut matched_existing = false;
        for model in &mut loaded.models {
            if model.get("id").and_then(Value::as_str) == Some(target_id.as_str()) {
                apply_managed_fields(model, target_id, &normalized_base_url, request);
                updated_entries += 1;
                matched_existing = true;
            }
        }
        if !matched_existing {
            loaded
                .models
                .push(new_managed_model(target_id, &normalized_base_url, request));
            created_entries += 1;
        }
    }

    let serialized = serialize_models(&loaded.models)?;

    if loaded.exists {
        write_credential_file_atomically(&paths.backup, &loaded.original_bytes)
            .map_err(|_| WorkBuddyError::new(WorkBuddyErrorCode::ConfigBackupFailed))?;
    } else if let Err(_error) = fs::create_dir_all(&paths.directory) {
        return Err(WorkBuddyError::new(WorkBuddyErrorCode::ConfigWriteFailed));
    }

    write_credential_file_atomically(&paths.models, &serialized)
        .map_err(|_| WorkBuddyError::new(WorkBuddyErrorCode::ConfigWriteFailed))?;

    Ok(SaveWorkBuddyModelsResult {
        revision: revision_for(&serialized),
        model_count: loaded.models.len(),
        created_entries,
        updated_entries,
        duplicate_ids,
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
                models: Vec::new(),
            });
        }
        Err(_) => return Err(WorkBuddyError::new(WorkBuddyErrorCode::ConfigReadFailed)),
    };

    let value: Value = serde_json::from_slice(&original_bytes)
        .map_err(|_| WorkBuddyError::new(WorkBuddyErrorCode::ConfigInvalidJson))?;
    let entries = value
        .as_array()
        .ok_or_else(|| WorkBuddyError::new(WorkBuddyErrorCode::ConfigRootNotArray))?;

    let mut models = Vec::with_capacity(entries.len());
    for (index, entry) in entries.iter().enumerate() {
        let model = entry.as_object().cloned().ok_or_else(|| {
            WorkBuddyError::new(WorkBuddyErrorCode::ConfigInvalidEntry)
                .with_invalid_entry_index(index)
        })?;
        let valid_id = model
            .get("id")
            .and_then(Value::as_str)
            .is_some_and(|id| !id.trim().is_empty());
        if !valid_id {
            return Err(WorkBuddyError::new(WorkBuddyErrorCode::ConfigInvalidEntry)
                .with_invalid_entry_index(index));
        }
        models.push(model);
    }

    Ok(LoadedConfig {
        exists: true,
        revision: Some(revision_for(&original_bytes)),
        original_bytes,
        models,
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

fn target_duplicate_ids(
    models: &[Map<String, Value>],
    target_ids: &[String],
) -> Vec<DuplicateModelId> {
    let target_set = target_ids
        .iter()
        .map(String::as_str)
        .collect::<HashSet<_>>();
    let mut counts = HashMap::<&str, usize>::new();
    for model in models {
        let Some(id) = model.get("id").and_then(Value::as_str) else {
            continue;
        };
        if target_set.contains(id) {
            *counts.entry(id).or_default() += 1;
        }
    }

    target_ids
        .iter()
        .filter_map(|id| {
            let count = counts.get(id.as_str()).copied().unwrap_or_default();
            (count > 1).then(|| DuplicateModelId {
                id: id.clone(),
                count,
            })
        })
        .collect()
}

fn new_managed_model(
    id: &str,
    normalized_base_url: &str,
    request: &SaveWorkBuddyModelsRequest,
) -> Map<String, Value> {
    let mut model = Map::new();
    apply_managed_fields(&mut model, id, normalized_base_url, request);
    model
}

fn apply_managed_fields(
    model: &mut Map<String, Value>,
    id: &str,
    normalized_base_url: &str,
    request: &SaveWorkBuddyModelsRequest,
) {
    let api_key = managed_api_key(model, request);
    model.insert("id".to_string(), Value::String(id.to_string()));
    model.insert("name".to_string(), Value::String(id.to_string()));
    model.insert("vendor".to_string(), Value::String("Custom".to_string()));
    model.insert(
        "url".to_string(),
        Value::String(normalized_base_url.to_string()),
    );
    model.insert("apiKey".to_string(), Value::String(api_key));
    model.insert("supportsToolCall".to_string(), Value::Bool(true));
    model.insert("supportsImages".to_string(), Value::Bool(true));
    model.insert("supportsReasoning".to_string(), Value::Bool(true));
    model.insert("useCustomProtocol".to_string(), Value::Bool(false));
    model.remove("onlyReasoning");

    if !model.get("reasoning").is_some_and(Value::is_object) {
        model.insert("reasoning".to_string(), Value::Object(Map::new()));
    }
    let Some(Value::Object(reasoning)) = model.get_mut("reasoning") else {
        return;
    };
    reasoning.insert(
        "defaultEffort".to_string(),
        Value::String("max".to_string()),
    );
    reasoning.insert(
        "supportedEfforts".to_string(),
        Value::Array(
            ["low", "medium", "high", "xhigh", "max"]
                .into_iter()
                .map(|effort| Value::String(effort.to_string()))
                .collect(),
        ),
    );
    reasoning.insert("canDisableThinking".to_string(), Value::Bool(false));
}

fn managed_api_key(model: &Map<String, Value>, request: &SaveWorkBuddyModelsRequest) -> String {
    if !request.api_key.trim().is_empty() {
        return request.api_key.clone();
    }
    if request.clear_existing_api_keys {
        return String::new();
    }
    model
        .get("apiKey")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

fn serialize_models(models: &[Map<String, Value>]) -> Result<Vec<u8>, WorkBuddyError> {
    let mut serialized = serde_json::to_vec_pretty(&Value::Array(
        models.iter().cloned().map(Value::Object).collect(),
    ))
    .map_err(|_| WorkBuddyError::new(WorkBuddyErrorCode::ConfigWriteFailed))?;
    serialized.push(b'\n');
    Ok(serialized)
}

fn revision_for(bytes: &[u8]) -> String {
    let mut mac = HmacSha256::new_from_slice(revision_mac_key())
        .expect("the fixed-size revision MAC key is always valid");
    mac.update(bytes);
    format!("{:x}", mac.finalize().into_bytes())
}

fn revision_mac_key() -> &'static [u8; 32] {
    static KEY: OnceLock<[u8; 32]> = OnceLock::new();
    KEY.get_or_init(|| {
        // Keep the secret process-local: public revision tokens remain opaque,
        // and a restarted renderer must refresh status before it can save.
        let mut key = [0u8; 32];
        key[..16].copy_from_slice(Uuid::new_v4().as_bytes());
        key[16..].copy_from_slice(Uuid::new_v4().as_bytes());
        key
    })
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
mod tests {
    use super::*;

    fn paths(temp: &tempfile::TempDir) -> WorkBuddyPaths {
        WorkBuddyPaths::from_home(temp.path())
    }

    fn request(expected_revision: Option<String>) -> SaveWorkBuddyModelsRequest {
        SaveWorkBuddyModelsRequest {
            base_url: "https://api.example.test".to_string(),
            api_key: "fake-api-key".to_string(),
            allow_no_api_key: false,
            selected_model_ids: vec!["model-a".to_string()],
            manual_model_ids: Vec::new(),
            clear_existing_api_keys: false,
            expected_revision,
            duplicate_policy: DuplicatePolicy::Reject,
        }
    }

    fn write_models(paths: &WorkBuddyPaths, contents: &str) {
        fs::create_dir_all(&paths.directory).unwrap();
        fs::write(&paths.models, contents.as_bytes()).unwrap();
    }

    fn read_json(paths: &WorkBuddyPaths) -> Value {
        serde_json::from_slice(&fs::read(&paths.models).unwrap()).unwrap()
    }

    fn raw_sha256_hex(bytes: &[u8]) -> String {
        use sha2::Digest;

        format!("{:x}", Sha256::digest(bytes))
    }

    #[test]
    fn first_save_creates_only_the_models_file_with_exact_managed_fields() {
        let temp = tempfile::tempdir().unwrap();
        let paths = paths(&temp);
        let result = save_workbuddy_models_at_locked(&paths, &request(None)).unwrap();

        assert_eq!(result.created_entries, 1);
        assert_eq!(result.updated_entries, 0);
        assert_eq!(result.model_count, 1);
        assert!(
            !paths.backup.exists(),
            "first creation must not create a backup"
        );
        let model = read_json(&paths)[0].as_object().unwrap().clone();
        assert_eq!(model.get("id"), Some(&Value::String("model-a".to_string())));
        assert_eq!(
            model.get("name"),
            Some(&Value::String("model-a".to_string()))
        );
        assert_eq!(
            model.get("vendor"),
            Some(&Value::String("Custom".to_string()))
        );
        assert_eq!(
            model.get("url"),
            Some(&Value::String("https://api.example.test/v1".to_string()))
        );
        assert_eq!(
            model.get("apiKey"),
            Some(&Value::String("fake-api-key".to_string()))
        );
        assert_eq!(model.get("supportsToolCall"), Some(&Value::Bool(true)));
        assert_eq!(model.get("supportsImages"), Some(&Value::Bool(true)));
        assert_eq!(model.get("supportsReasoning"), Some(&Value::Bool(true)));
        assert_eq!(model.get("useCustomProtocol"), Some(&Value::Bool(false)));
        assert_eq!(
            model.get("reasoning"),
            Some(&serde_json::json!({
                "defaultEffort": "max",
                "supportedEfforts": ["low", "medium", "high", "xhigh", "max"],
                "canDisableThinking": false
            }))
        );
    }

    #[test]
    fn allowed_empty_key_creates_a_new_entry_with_an_empty_api_key() {
        let temp = tempfile::tempdir().unwrap();
        let paths = paths(&temp);
        let mut no_key_request = request(None);
        no_key_request.api_key.clear();
        no_key_request.allow_no_api_key = true;

        save_workbuddy_models_at_locked(&paths, &no_key_request).unwrap();
        assert_eq!(read_json(&paths)[0]["apiKey"], "");
    }

    #[test]
    fn status_is_a_summary_without_models_or_keys() {
        let temp = tempfile::tempdir().unwrap();
        let paths = paths(&temp);
        let missing = get_workbuddy_status_at(&paths).unwrap();
        assert!(!missing.exists);
        assert_eq!(missing.model_count, 0);
        assert!(missing.revision.is_none());

        save_workbuddy_models_at_locked(&paths, &request(None)).unwrap();
        let status = get_workbuddy_status_at(&paths).unwrap();
        let serialized = serde_json::to_string(&status).unwrap();
        assert!(status.exists);
        assert_eq!(status.model_count, 1);
        assert!(status.revision.is_some());
        assert!(!serialized.contains("fake-api-key"));
        assert!(!serialized.contains("model-a"));
    }

    #[test]
    fn revision_is_an_opaque_mac_not_a_raw_api_key_dictionary_digest() {
        let temp = tempfile::tempdir().unwrap();
        let paths = paths(&temp);
        let original = br#"[{"id":"model-a","apiKey":"correct-horse-battery-staple"}]"#;
        write_models(&paths, std::str::from_utf8(original).unwrap());

        let status = get_workbuddy_status_at(&paths).unwrap();
        let serialized_status = serde_json::to_string(&status).unwrap();
        let revision = status.revision.unwrap();

        assert_eq!(revision.len(), 64);
        assert!(!serialized_status.contains("correct-horse-battery-staple"));
        assert_ne!(revision, raw_sha256_hex(original));
        for candidate_key in ["correct-horse-battery-staple", "wrong-key", ""] {
            let candidate = format!(r#"[{{"id":"model-a","apiKey":"{candidate_key}"}}]"#);
            assert_ne!(
                revision,
                raw_sha256_hex(candidate.as_bytes()),
                "a public revision must not support a direct SHA-256 key dictionary comparison"
            );
        }
    }

    #[test]
    fn api_key_only_external_change_invalidates_revision_without_a_blind_write() {
        let temp = tempfile::tempdir().unwrap();
        let paths = paths(&temp);
        write_models(&paths, r#"[{"id":"model-a","apiKey":"first-secret"}]"#);
        let stale_revision = get_workbuddy_status_at(&paths).unwrap().revision.unwrap();

        fs::write(
            &paths.models,
            r#"[{"id":"model-a","apiKey":"externally-rotated-secret"}]"#,
        )
        .unwrap();
        let before = fs::read(&paths.models).unwrap();
        let current_revision = get_workbuddy_status_at(&paths).unwrap().revision.unwrap();
        assert_ne!(stale_revision, current_revision);

        let error = save_workbuddy_models_at_locked(&paths, &request(Some(stale_revision)))
            .expect_err("an API-key-only external change must reject the stale save");
        assert_eq!(
            error.code(),
            WorkBuddyErrorCode::ConfigConcurrentModification
        );
        assert_eq!(fs::read(&paths.models).unwrap(), before);
        assert!(
            !paths.backup.exists(),
            "a stale revision must fail before creating a credential backup"
        );
    }

    #[test]
    fn updates_preserve_unknown_fields_reasoning_extra_and_position() {
        let temp = tempfile::tempdir().unwrap();
        let paths = paths(&temp);
        let original = r#"[
  {
    "id": "model-a",
    "apiKey": "old-key",
    "onlyReasoning": true,
    "futureField": {"kept": true},
    "reasoning": {"futureReasoningField": 42, "defaultEffort": "low"}
  },
  {"id": "other-model", "future": "untouched"}
]"#;
        write_models(&paths, original);
        let before = fs::read(&paths.models).unwrap();
        let revision = get_workbuddy_status_at(&paths).unwrap().revision;

        let result = save_workbuddy_models_at_locked(&paths, &request(revision)).unwrap();
        assert_eq!(result.created_entries, 0);
        assert_eq!(result.updated_entries, 1);
        assert_eq!(fs::read(&paths.backup).unwrap(), before);

        let models = read_json(&paths).as_array().unwrap().clone();
        assert_eq!(models[0]["id"], "model-a");
        assert_eq!(models[0]["futureField"], serde_json::json!({"kept": true}));
        assert_eq!(models[0]["reasoning"]["futureReasoningField"], 42);
        assert_eq!(models[0]["reasoning"]["defaultEffort"], "max");
        assert!(models[0].get("onlyReasoning").is_none());
        assert_eq!(
            models[1],
            serde_json::json!({"id": "other-model", "future": "untouched"})
        );
    }

    #[test]
    fn empty_key_preserves_each_existing_key_unless_explicitly_cleared() {
        let temp = tempfile::tempdir().unwrap();
        let paths = paths(&temp);
        write_models(
            &paths,
            r#"[
              {"id":"model-a","apiKey":"first-key"},
              {"id":"model-a","apiKey":"second-key"}
            ]"#,
        );
        let revision = get_workbuddy_status_at(&paths).unwrap().revision;
        let mut keep_request = request(revision);
        keep_request.api_key.clear();
        keep_request.allow_no_api_key = true;
        keep_request.duplicate_policy = DuplicatePolicy::UpdateAll;
        let result = save_workbuddy_models_at_locked(&paths, &keep_request).unwrap();
        assert_eq!(result.updated_entries, 2);
        assert_eq!(
            result.duplicate_ids,
            vec![DuplicateModelId {
                id: "model-a".to_string(),
                count: 2
            }]
        );
        let kept = read_json(&paths);
        assert_eq!(kept[0]["apiKey"], "first-key");
        assert_eq!(kept[1]["apiKey"], "second-key");

        let mut clear_request = request(Some(result.revision));
        clear_request.api_key.clear();
        clear_request.allow_no_api_key = true;
        clear_request.clear_existing_api_keys = true;
        clear_request.duplicate_policy = DuplicatePolicy::UpdateAll;
        save_workbuddy_models_at_locked(&paths, &clear_request).unwrap();
        let cleared = read_json(&paths);
        assert_eq!(cleared[0]["apiKey"], "");
        assert_eq!(cleared[1]["apiKey"], "");
    }

    #[test]
    fn duplicate_reject_does_not_create_backup_or_modify_the_file() {
        let temp = tempfile::tempdir().unwrap();
        let paths = paths(&temp);
        write_models(&paths, r#"[{"id":"model-a"},{"id":"model-a"}]"#);
        let before = fs::read(&paths.models).unwrap();
        let revision = get_workbuddy_status_at(&paths).unwrap().revision;

        let error = save_workbuddy_models_at_locked(&paths, &request(revision)).unwrap_err();
        assert_eq!(error.code(), WorkBuddyErrorCode::ConfigDuplicateTarget);
        assert_eq!(
            error.to_dto().details.duplicate_ids,
            vec![DuplicateModelId {
                id: "model-a".to_string(),
                count: 2,
            }]
        );
        assert_eq!(fs::read(&paths.models).unwrap(), before);
        assert!(!paths.backup.exists());
    }

    #[test]
    fn stale_revision_and_invalid_json_do_not_overwrite_or_backup() {
        let temp = tempfile::tempdir().unwrap();
        let paths = paths(&temp);
        write_models(&paths, r#"[{"id":"model-a"}]"#);
        let stale = get_workbuddy_status_at(&paths).unwrap().revision;
        fs::write(&paths.models, r#"[{"id":"outside-change"}]"#).unwrap();
        let before_conflict = fs::read(&paths.models).unwrap();
        let error = save_workbuddy_models_at_locked(&paths, &request(stale)).unwrap_err();
        assert_eq!(
            error.code(),
            WorkBuddyErrorCode::ConfigConcurrentModification
        );
        assert_eq!(fs::read(&paths.models).unwrap(), before_conflict);
        assert!(!paths.backup.exists());

        fs::write(&paths.models, b"{ not-json").unwrap();
        let before_invalid = fs::read(&paths.models).unwrap();
        let invalid_error = save_workbuddy_models_at_locked(&paths, &request(None)).unwrap_err();
        assert_eq!(invalid_error.code(), WorkBuddyErrorCode::ConfigInvalidJson);
        assert_eq!(fs::read(&paths.models).unwrap(), before_invalid);
        assert!(!paths.backup.exists());
    }

    #[test]
    fn invalid_root_and_entries_fail_without_silent_repair() {
        let temp = tempfile::tempdir().unwrap();
        let paths = paths(&temp);
        write_models(&paths, r#"{"not":"an-array"}"#);
        assert_eq!(
            get_workbuddy_status_at(&paths).unwrap_err().code(),
            WorkBuddyErrorCode::ConfigRootNotArray
        );

        write_models(&paths, r#"[{"id":"ok"}, 7]"#);
        let error = get_workbuddy_status_at(&paths).unwrap_err();
        assert_eq!(error.code(), WorkBuddyErrorCode::ConfigInvalidEntry);
        assert_eq!(error.to_dto().details.invalid_entry_index, Some(1));

        write_models(&paths, r#"[{"id":"   "}]"#);
        assert_eq!(
            get_workbuddy_status_at(&paths).unwrap_err().code(),
            WorkBuddyErrorCode::ConfigInvalidEntry
        );
    }

    #[test]
    fn target_order_is_selected_then_manual_with_case_sensitive_deduplication() {
        let temp = tempfile::tempdir().unwrap();
        let paths = paths(&temp);
        let mut request = request(None);
        request.selected_model_ids = vec!["b".to_string(), "A".to_string(), "b".to_string()];
        request.manual_model_ids = vec![" A ".to_string(), "a".to_string(), "\n".to_string()];
        request.api_key = "fake".to_string();

        save_workbuddy_models_at_locked(&paths, &request).unwrap();
        let ids = read_json(&paths)
            .as_array()
            .unwrap()
            .iter()
            .map(|model| model["id"].as_str().unwrap().to_string())
            .collect::<Vec<_>>();
        assert_eq!(ids, ["b", "A", "a"]);
    }

    #[test]
    fn backup_is_replaced_with_the_immediately_previous_valid_file() {
        let temp = tempfile::tempdir().unwrap();
        let paths = paths(&temp);
        write_models(&paths, r#"[{"id":"old"}]"#);
        let original = fs::read(&paths.models).unwrap();
        let first_revision = get_workbuddy_status_at(&paths).unwrap().revision;
        let mut first = request(first_revision);
        first.selected_model_ids = vec!["first".to_string()];
        let first_result = save_workbuddy_models_at_locked(&paths, &first).unwrap();
        assert_eq!(fs::read(&paths.backup).unwrap(), original);

        let first_main = fs::read(&paths.models).unwrap();
        let mut second = request(Some(first_result.revision));
        second.selected_model_ids = vec!["second".to_string()];
        save_workbuddy_models_at_locked(&paths, &second).unwrap();
        assert_eq!(fs::read(&paths.backup).unwrap(), first_main);
    }

    #[test]
    fn backup_failure_leaves_the_primary_file_untouched() {
        let temp = tempfile::tempdir().unwrap();
        let paths = paths(&temp);
        write_models(&paths, r#"[{"id":"model-a"}]"#);
        fs::create_dir(&paths.backup).unwrap();
        let before = fs::read(&paths.models).unwrap();
        let revision = get_workbuddy_status_at(&paths).unwrap().revision;

        let error = save_workbuddy_models_at_locked(&paths, &request(revision)).unwrap_err();
        assert_eq!(error.code(), WorkBuddyErrorCode::ConfigBackupFailed);
        assert_eq!(fs::read(&paths.models).unwrap(), before);
    }

    #[tokio::test]
    async fn concurrent_saves_are_serialized_and_stale_revision_loses() {
        let temp = tempfile::tempdir().unwrap();
        let paths = paths(&temp);
        let (first, second) = tokio::join!(
            save_workbuddy_models_at(paths.clone(), request(None)),
            save_workbuddy_models_at(paths.clone(), request(None)),
        );

        assert!(first.is_ok() ^ second.is_ok());
        let error = first.err().or_else(|| second.err()).unwrap();
        assert_eq!(
            error.code(),
            WorkBuddyErrorCode::ConfigConcurrentModification
        );
        assert_eq!(read_json(&paths).as_array().unwrap().len(), 1);
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

    #[cfg(windows)]
    #[test]
    fn existing_windows_target_uses_the_strict_replace_primitive() {
        let temp = tempfile::tempdir().unwrap();
        let target = temp.path().join("models.json");
        fs::write(&target, b"old").unwrap();

        write_credential_file_atomically(&target, b"new").unwrap();

        assert_eq!(fs::read(&target).unwrap(), b"new");
    }

    #[cfg(unix)]
    #[test]
    fn credential_files_are_created_with_user_only_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let temp = tempfile::tempdir().unwrap();
        let paths = paths(&temp);
        let first = save_workbuddy_models_at_locked(&paths, &request(None)).unwrap();
        assert_eq!(
            fs::metadata(&paths.models).unwrap().permissions().mode() & 0o077,
            0
        );

        let mut second = request(Some(first.revision));
        second.selected_model_ids = vec!["model-b".to_string()];
        save_workbuddy_models_at_locked(&paths, &second).unwrap();
        assert_eq!(
            fs::metadata(&paths.models).unwrap().permissions().mode() & 0o077,
            0
        );
        assert_eq!(
            fs::metadata(&paths.backup).unwrap().permissions().mode() & 0o077,
            0
        );
    }
}
