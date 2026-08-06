use std::collections::HashSet;
use std::path::{Path, PathBuf};

use rusqlite::Connection;
use serde_json::Value;

use crate::session_manager::terminal::session_resume_argument;
use crate::session_manager::{SessionMessage, SessionMeta};

use super::utils::{parse_timestamp_to_ms, path_basename, truncate_summary};

const PROVIDER_ID: &str = "opencode";

/// Return the OpenCode base directory (`$XDG_DATA_HOME/opencode`).
///
/// Respects `XDG_DATA_HOME` on all platforms; falls back to
/// `~/.local/share/opencode/`.
pub(crate) fn get_opencode_base_dir() -> PathBuf {
    if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
        if !xdg.is_empty() {
            return PathBuf::from(xdg).join("opencode");
        }
    }
    dirs::home_dir()
        .map(|h| h.join(".local/share/opencode"))
        .unwrap_or_else(|| PathBuf::from(".local/share/opencode"))
}

/// Return the OpenCode JSON storage directory (legacy flat-file layout).
pub(crate) fn get_opencode_data_dir() -> PathBuf {
    get_opencode_base_dir().join("storage")
}

fn get_opencode_db_path() -> PathBuf {
    get_opencode_base_dir().join("opencode.db")
}

/// Scan sessions from both the legacy JSON files and the newer SQLite database,
/// merging results with SQLite taking precedence on ID conflicts.
pub fn scan_sessions() -> Vec<SessionMeta> {
    let json_sessions = scan_sessions_json();
    let sqlite_sessions = scan_sessions_sqlite();

    if sqlite_sessions.is_empty() {
        return json_sessions;
    }
    if json_sessions.is_empty() {
        return sqlite_sessions;
    }

    // Deduplicate: keep SQLite version when the same session_id exists in both
    let sqlite_ids: std::collections::HashSet<String> = sqlite_sessions
        .iter()
        .map(|s| s.session_id.clone())
        .collect();

    let mut merged = sqlite_sessions;
    for s in json_sessions {
        if !sqlite_ids.contains(&s.session_id) {
            merged.push(s);
        }
    }
    merged
}

fn scan_sessions_json() -> Vec<SessionMeta> {
    let storage = get_opencode_data_dir();
    let storage = match validate_storage_root(&storage) {
        Ok(storage) => storage,
        Err(_) => return Vec::new(),
    };
    let session_dir = match validate_directory_root(&storage, "session", "OpenCode session") {
        Ok(Some(session_dir)) => session_dir,
        Ok(None) | Err(_) => return Vec::new(),
    };

    let mut json_files = Vec::new();
    if collect_json_files_checked(&session_dir, &mut json_files).is_err() {
        return Vec::new();
    }

    let mut sessions = Vec::new();
    for path in json_files {
        if let Some(meta) = parse_session(&storage, &path) {
            sessions.push(meta);
        }
    }
    sessions
}

/// Parse a SQLite source reference in the format `sqlite:<db_path>:<session_id>`.
///
/// Uses `rfind(":ses_")` to split the path from the session ID because the
/// db path itself may contain colons (e.g. `C:\Users\...` on Windows).
/// This relies on the OpenCode convention that session IDs start with `ses_`.
fn parse_sqlite_source(source: &str) -> Option<(PathBuf, String)> {
    let rest = source.strip_prefix("sqlite:")?;
    let sep = rest.rfind(":ses_")?;
    let db_path = PathBuf::from(&rest[..sep]);
    let session_id = rest[sep + 1..].to_string();
    Some((db_path, session_id))
}

fn scan_sessions_sqlite() -> Vec<SessionMeta> {
    let db_path = get_opencode_db_path();
    if !db_path.exists() {
        return Vec::new();
    }

    let conn = match Connection::open_with_flags(
        &db_path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    ) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    let mut stmt = match conn.prepare(
        "SELECT id, title, directory, time_created, time_updated FROM session ORDER BY time_updated DESC",
    ) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };

    let db_display = db_path.display().to_string();

    let iter = match stmt.query_map([], |row| {
        let session_id: String = row.get(0)?;
        let title: String = row.get(1)?;
        let directory: String = row.get(2)?;
        let created: i64 = row.get(3)?;
        let updated: i64 = row.get(4)?;
        Ok((session_id, title, directory, created, updated))
    }) {
        Ok(rows) => rows,
        Err(_) => return Vec::new(),
    };

    let mut sessions = Vec::new();
    for row in iter.flatten() {
        let (session_id, title, directory, created, updated) = row;
        let display_title = if title.is_empty() {
            path_basename(&directory)
        } else {
            Some(title)
        };
        sessions.push(SessionMeta {
            provider_id: PROVIDER_ID.to_string(),
            session_id: session_id.clone(),
            title: display_title.clone(),
            summary: display_title,
            project_dir: if directory.is_empty() {
                None
            } else {
                Some(directory)
            },
            created_at: Some(created),
            last_active_at: Some(updated),
            source_path: Some(format!("sqlite:{db_display}:{session_id}")),
            resume_command: session_resume_argument(&session_id)
                .map(|argument| format!("opencode -s {argument}")),
        });
    }
    sessions
}

pub fn load_messages(path: &Path) -> Result<Vec<SessionMessage>, String> {
    // `path` is the message directory: storage/message/{sessionID}/
    if !path.is_dir() {
        return Err(format!("Message directory not found: {}", path.display()));
    }

    let storage = path
        .parent()
        .and_then(|p| p.parent())
        .ok_or_else(|| "Cannot determine storage root from message path".to_string())?;
    let storage = validate_storage_root(storage)?;
    let session_id = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "OpenCode message directory has no UTF-8 session ID".to_string())?;
    if !is_safe_storage_id(session_id) {
        return Err(format!("Invalid OpenCode session ID: {session_id:?}"));
    }
    let path = validate_message_source(&storage, path, session_id)?;

    let mut msg_files = Vec::new();
    collect_json_files_checked(&path, &mut msg_files)?;

    // Parse all messages and collect (created_ts, message_id, role, parts_text)
    let mut entries: Vec<(i64, String, String, String)> = Vec::new();

    for msg_path in &msg_files {
        let data = match std::fs::read_to_string(msg_path) {
            Ok(d) => d,
            Err(_) => continue,
        };
        let value: Value = match serde_json::from_str(&data) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let msg_id = match value.get("id").and_then(Value::as_str) {
            Some(id) => id.to_string(),
            None => continue,
        };
        if !is_safe_storage_id(&msg_id) {
            return Err(format!("Invalid OpenCode message ID: {msg_id:?}"));
        }

        let role = value
            .get("role")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string();

        let created_ts = value
            .get("time")
            .and_then(|t| t.get("created"))
            .and_then(parse_timestamp_to_ms)
            .unwrap_or(0);

        // Collect text parts from storage/part/{messageID}/
        let part_dir = validate_part_targets(&storage, std::slice::from_ref(&msg_id))?
            .into_iter()
            .next();
        let text = match part_dir {
            Some(part_dir) => collect_parts_text(&part_dir)?,
            None => String::new(),
        };
        if text.trim().is_empty() {
            continue;
        }

        entries.push((created_ts, msg_id, role, text));
    }

    // Sort by created timestamp
    entries.sort_by_key(|(ts, _, _, _)| *ts);

    let messages = entries
        .into_iter()
        .map(|(ts, _, role, content)| SessionMessage {
            role,
            content,
            ts: if ts > 0 { Some(ts) } else { None },
        })
        .collect();

    Ok(messages)
}

/// Load messages from the OpenCode SQLite database for a given source reference.
/// Joins the `message` and `part` tables in memory to reconstruct full messages.
pub fn load_messages_sqlite(source: &str) -> Result<Vec<SessionMessage>, String> {
    let (db_path, session_id) = parse_sqlite_source(source)
        .ok_or_else(|| format!("Invalid SQLite source reference: {source}"))?;

    let conn = Connection::open_with_flags(
        &db_path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|e| format!("Failed to open OpenCode database: {e}"))?;

    let mut msg_stmt = conn
        .prepare(
            "SELECT id, time_created, data FROM message WHERE session_id = ?1 ORDER BY time_created ASC",
        )
        .map_err(|e| format!("Failed to prepare message query: {e}"))?;

    let msg_rows = msg_stmt
        .query_map([session_id.as_str()], |row| {
            let id: String = row.get(0)?;
            let ts: i64 = row.get(1)?;
            let data: String = row.get(2)?;
            Ok((id, ts, data))
        })
        .map_err(|e| format!("Failed to query messages: {e}"))?;

    let mut part_stmt = conn
        .prepare(
            "SELECT message_id, data FROM part WHERE session_id = ?1 ORDER BY time_created ASC",
        )
        .map_err(|e| format!("Failed to prepare part query: {e}"))?;

    let part_rows = part_stmt
        .query_map([session_id.as_str()], |row| {
            let message_id: String = row.get(0)?;
            let data: String = row.get(1)?;
            Ok((message_id, data))
        })
        .map_err(|e| format!("Failed to query parts: {e}"))?;

    let mut parts_map: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    for part in part_rows.flatten() {
        let (message_id, data) = part;
        parts_map.entry(message_id).or_default().push(data);
    }

    let mut messages = Vec::new();
    for row in msg_rows.flatten() {
        let (msg_id, ts, data) = row;
        let msg_value: Value = match serde_json::from_str(&data) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let role = msg_value
            .get("role")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string();

        let mut texts = Vec::new();
        if let Some(parts) = parts_map.get(&msg_id) {
            for part_data in parts {
                let part_value: Value = match serde_json::from_str(part_data) {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                if let Some(text) = extract_part_text(&part_value) {
                    texts.push(text);
                }
            }
        }

        let content = texts.join("\n");
        if content.trim().is_empty() {
            continue;
        }

        messages.push(SessionMessage {
            role,
            content,
            ts: Some(ts),
        });
    }

    Ok(messages)
}

pub fn delete_session(storage: &Path, path: &Path, session_id: &str) -> Result<bool, String> {
    if !is_safe_storage_id(session_id) {
        return Err(format!(
            "Invalid OpenCode session ID for deletion: {session_id:?}"
        ));
    }

    let storage = validate_storage_root(storage)?;
    let path = validate_message_source(&storage, path, session_id)?;

    if path.file_name().and_then(|name| name.to_str()) != Some(session_id) {
        return Err(format!(
            "OpenCode session path does not match session ID: expected {session_id}, found {}",
            path.display()
        ));
    }

    let mut message_files = Vec::new();
    collect_json_files_checked(&path, &mut message_files)?;

    let mut message_ids = HashSet::new();
    for message_path in &message_files {
        let data = match std::fs::read_to_string(message_path) {
            Ok(data) => data,
            Err(_) => continue,
        };
        let value: Value = match serde_json::from_str(&data) {
            Ok(value) => value,
            Err(_) => continue,
        };
        if let Some(message_id) = value.get("id").and_then(Value::as_str) {
            if !is_safe_storage_id(message_id) {
                return Err(format!(
                    "Invalid OpenCode message ID for deletion: {message_id:?}"
                ));
            }
            message_ids.insert(message_id.to_string());
        }
    }

    let mut message_ids = message_ids.into_iter().collect::<Vec<_>>();
    message_ids.sort_unstable();

    // Resolve and validate every derived target before the first mutation. A
    // malformed later message must not turn deletion into a partial operation.
    let part_dirs = validate_part_targets(&storage, &message_ids)?;
    let session_diff_path = validate_optional_file_target(
        &storage,
        "session_diff",
        &format!("{session_id}.json"),
        "OpenCode session diff",
    )?;
    let session_file = find_session_file(&storage, session_id)?;

    for part_dir in &part_dirs {
        remove_dir_all_if_exists(part_dir).map_err(|e| {
            format!(
                "Failed to delete OpenCode part directory {}: {e}",
                part_dir.display()
            )
        })?;
    }

    if let Some(session_diff_path) = session_diff_path {
        remove_file_if_exists(&session_diff_path).map_err(|e| {
            format!(
                "Failed to delete OpenCode session diff {}: {e}",
                session_diff_path.display()
            )
        })?;
    }

    remove_dir_all_if_exists(&path).map_err(|e| {
        format!(
            "Failed to delete OpenCode message directory {}: {e}",
            path.display()
        )
    })?;

    if let Some(session_file) = session_file {
        remove_file_if_exists(&session_file).map_err(|e| {
            format!(
                "Failed to delete OpenCode session file {}: {e}",
                session_file.display()
            )
        })?;
    }

    Ok(true)
}

/// Delete a session from the OpenCode SQLite database.
pub fn delete_session_sqlite(session_id: &str, source: &str) -> Result<bool, String> {
    let (db_path, ref_session_id) = parse_sqlite_source(source)
        .ok_or_else(|| format!("Invalid SQLite source reference: {source}"))?;
    let db_path = db_path
        .canonicalize()
        .map_err(|e| format!("Failed to canonicalize SQLite database path: {e}"))?;
    let expected_db_path = get_opencode_db_path()
        .canonicalize()
        .map_err(|e| format!("Failed to canonicalize expected OpenCode database path: {e}"))?;

    if ref_session_id != session_id {
        return Err(format!(
            "OpenCode SQLite session ID mismatch: expected {session_id}, found {ref_session_id}"
        ));
    }
    if db_path != expected_db_path {
        return Err("SQLite path does not match expected OpenCode database".to_string());
    }

    let conn =
        Connection::open(&db_path).map_err(|e| format!("Failed to open OpenCode database: {e}"))?;

    let tx = conn
        .unchecked_transaction()
        .map_err(|e| format!("Failed to begin transaction: {e}"))?;

    tx.execute("DELETE FROM part WHERE session_id = ?1", [session_id])
        .map_err(|e| format!("Failed to delete OpenCode parts: {e}"))?;
    tx.execute("DELETE FROM message WHERE session_id = ?1", [session_id])
        .map_err(|e| format!("Failed to delete OpenCode messages: {e}"))?;

    let deleted = tx
        .execute("DELETE FROM session WHERE id = ?1", [session_id])
        .map_err(|e| format!("Failed to delete OpenCode session: {e}"))?;

    tx.commit()
        .map_err(|e| format!("Failed to commit session deletion: {e}"))?;

    Ok(deleted > 0)
}

fn parse_session(storage: &Path, path: &Path) -> Option<SessionMeta> {
    let data = std::fs::read_to_string(path).ok()?;
    let value: Value = serde_json::from_str(&data).ok()?;

    let session_id = value.get("id").and_then(Value::as_str)?.to_string();
    let title = value
        .get("title")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());
    let directory = value
        .get("directory")
        .and_then(Value::as_str)
        .map(|s| s.to_string());

    let created_at = value
        .get("time")
        .and_then(|t| t.get("created"))
        .and_then(parse_timestamp_to_ms);
    let updated_at = value
        .get("time")
        .and_then(|t| t.get("updated"))
        .and_then(parse_timestamp_to_ms);

    // Derive title from directory basename if no explicit title
    let has_title = title.is_some();
    let display_title = title.or_else(|| {
        directory
            .as_deref()
            .and_then(path_basename)
            .map(|s| s.to_string())
    });

    let safe_session_id = is_safe_storage_id(&session_id);
    let source_path = safe_session_id.then(|| {
        storage
            .join("message")
            .join(&session_id)
            .to_string_lossy()
            .to_string()
    });

    // Skip expensive I/O if title already available from session JSON
    let summary = if has_title {
        display_title.clone()
    } else if !safe_session_id {
        None
    } else {
        get_first_user_summary(storage, &session_id)
    };

    Some(SessionMeta {
        provider_id: PROVIDER_ID.to_string(),
        session_id: session_id.clone(),
        title: display_title,
        summary,
        project_dir: directory,
        created_at,
        last_active_at: updated_at.or(created_at),
        source_path,
        resume_command: session_resume_argument(&session_id)
            .map(|argument| format!("opencode -s {argument}")),
    })
}

/// Read the first user message's first text part to use as summary.
fn get_first_user_summary(storage: &Path, session_id: &str) -> Option<String> {
    if !is_safe_storage_id(session_id) {
        return None;
    }
    let msg_dir = storage.join("message").join(session_id);
    let messages = load_messages(&msg_dir).ok()?;
    let first_user = messages
        .iter()
        .find(|message| message.role == "user" && !message.content.trim().is_empty())?;
    Some(truncate_summary(&first_user.content, 160))
}

/// Collect text content from all parts in a part directory.
fn extract_part_text(part_value: &Value) -> Option<String> {
    match part_value.get("type").and_then(Value::as_str) {
        Some("text") => part_value
            .get("text")
            .and_then(Value::as_str)
            .filter(|t| !t.trim().is_empty())
            .map(|t| t.to_string()),
        Some("tool") => {
            let tool = part_value
                .get("tool")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            Some(format!("[Tool: {tool}]"))
        }
        _ => None,
    }
}

fn collect_parts_text(part_dir: &Path) -> Result<String, String> {
    if !part_dir.is_dir() {
        return Ok(String::new());
    }

    let mut parts = Vec::new();
    collect_json_files_checked(part_dir, &mut parts)?;

    let mut texts = Vec::new();
    for part_path in &parts {
        let data = match std::fs::read_to_string(part_path) {
            Ok(d) => d,
            Err(_) => continue,
        };
        let value: Value = match serde_json::from_str(&data) {
            Ok(v) => v,
            Err(_) => continue,
        };

        if let Some(text) = extract_part_text(&value) {
            texts.push(text);
        }
    }

    Ok(texts.join("\n"))
}

fn is_safe_storage_id(value: &str) -> bool {
    let mut bytes = value.bytes();
    bytes
        .next()
        .is_some_and(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
}

fn validate_storage_root(storage: &Path) -> Result<PathBuf, String> {
    let metadata = std::fs::symlink_metadata(storage)
        .map_err(|e| format!("Failed to inspect OpenCode storage root: {e}"))?;
    if metadata.file_type().is_symlink() {
        return Err("OpenCode storage root must not be a symlink".to_string());
    }
    if !metadata.is_dir() {
        return Err("OpenCode storage root is not a directory".to_string());
    }
    storage
        .canonicalize()
        .map_err(|e| format!("Failed to canonicalize OpenCode storage root: {e}"))
}

fn validate_directory_root(
    storage: &Path,
    name: &str,
    label: &str,
) -> Result<Option<PathBuf>, String> {
    let path = storage.join(name);
    let metadata = match std::fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("Failed to inspect {label} root: {error}")),
    };
    if metadata.file_type().is_symlink() {
        return Err(format!("{label} root must not be a symlink"));
    }
    if !metadata.is_dir() {
        return Err(format!("{label} root is not a directory"));
    }

    let canonical = path
        .canonicalize()
        .map_err(|e| format!("Failed to canonicalize {label} root: {e}"))?;
    if canonical == storage || !canonical.starts_with(storage) {
        return Err(format!("{label} root is outside OpenCode storage"));
    }
    Ok(Some(canonical))
}

fn validate_message_source(
    storage: &Path,
    path: &Path,
    session_id: &str,
) -> Result<PathBuf, String> {
    let message_root = validate_directory_root(storage, "message", "OpenCode message")?
        .ok_or_else(|| "OpenCode message root does not exist".to_string())?;
    let metadata = std::fs::symlink_metadata(path)
        .map_err(|e| format!("Failed to inspect OpenCode message directory: {e}"))?;
    if metadata.file_type().is_symlink() {
        return Err("OpenCode message directory must not be a symlink".to_string());
    }
    if !metadata.is_dir() {
        return Err("OpenCode message source is not a directory".to_string());
    }

    let canonical = path
        .canonicalize()
        .map_err(|e| format!("Failed to canonicalize OpenCode message directory: {e}"))?;
    if canonical.parent() != Some(message_root.as_path())
        || canonical.file_name().and_then(|name| name.to_str()) != Some(session_id)
    {
        return Err(format!(
            "OpenCode message directory is outside its session slot: {}",
            path.display()
        ));
    }
    Ok(canonical)
}

fn collect_json_files_checked(root: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    let entries = std::fs::read_dir(root).map_err(|e| {
        format!(
            "Failed to read OpenCode storage directory {}: {e}",
            root.display()
        )
    })?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to inspect OpenCode message entry: {e}"))?;
        let path = entry.path();
        let file_type = entry.file_type().map_err(|e| {
            format!(
                "Failed to inspect OpenCode message entry {}: {e}",
                path.display()
            )
        })?;
        if file_type.is_symlink() {
            return Err(format!(
                "OpenCode message tree must not contain a symlink: {}",
                path.display()
            ));
        }
        if file_type.is_dir() {
            collect_json_files_checked(&path, files)?;
        } else if file_type.is_file()
            && path.extension().and_then(|extension| extension.to_str()) == Some("json")
        {
            files.push(path);
        }
    }
    Ok(())
}

fn validate_part_targets(storage: &Path, message_ids: &[String]) -> Result<Vec<PathBuf>, String> {
    let Some(part_root) = validate_directory_root(storage, "part", "OpenCode part")? else {
        return Ok(Vec::new());
    };
    let mut targets = Vec::new();

    for message_id in message_ids {
        let target = part_root.join(message_id);
        let metadata = match std::fs::symlink_metadata(&target) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => {
                return Err(format!(
                    "Failed to inspect OpenCode part directory {}: {error}",
                    target.display()
                ));
            }
        };
        if metadata.file_type().is_symlink() {
            return Err(format!(
                "OpenCode part directory must not be a symlink: {}",
                target.display()
            ));
        }
        if !metadata.is_dir() {
            return Err(format!(
                "OpenCode part target is not a directory: {}",
                target.display()
            ));
        }

        let canonical = target.canonicalize().map_err(|e| {
            format!(
                "Failed to canonicalize OpenCode part directory {}: {e}",
                target.display()
            )
        })?;
        if !canonical.starts_with(&part_root) || canonical.parent() != Some(part_root.as_path()) {
            return Err(format!(
                "OpenCode part directory is outside the part root: {}",
                target.display()
            ));
        }
        targets.push(canonical);
    }
    Ok(targets)
}

fn validate_optional_file_target(
    storage: &Path,
    root_name: &str,
    filename: &str,
    label: &str,
) -> Result<Option<PathBuf>, String> {
    let Some(root) = validate_directory_root(storage, root_name, label)? else {
        return Ok(None);
    };
    let target = root.join(filename);
    let metadata = match std::fs::symlink_metadata(&target) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("Failed to inspect {label} target: {error}")),
    };
    if metadata.file_type().is_symlink() {
        return Err(format!("{label} target must not be a symlink"));
    }
    if !metadata.is_file() {
        return Err(format!("{label} target is not a file"));
    }
    let canonical = target
        .canonicalize()
        .map_err(|e| format!("Failed to canonicalize {label} target: {e}"))?;
    if canonical.parent() != Some(root.as_path()) {
        return Err(format!("{label} target is outside its storage root"));
    }
    Ok(Some(canonical))
}

fn find_session_file(storage: &Path, session_id: &str) -> Result<Option<PathBuf>, String> {
    let Some(session_root) = validate_directory_root(storage, "session", "OpenCode session")?
    else {
        return Ok(None);
    };
    let expected = format!("{session_id}.json");
    find_session_file_in_root(&session_root, &session_root, &expected)
}

fn find_session_file_in_root(
    root: &Path,
    current: &Path,
    expected: &str,
) -> Result<Option<PathBuf>, String> {
    let entries = std::fs::read_dir(current).map_err(|e| {
        format!(
            "Failed to read OpenCode session directory {}: {e}",
            current.display()
        )
    })?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to inspect OpenCode session entry: {e}"))?;
        let path = entry.path();
        let file_type = entry.file_type().map_err(|e| {
            format!(
                "Failed to inspect OpenCode session entry {}: {e}",
                path.display()
            )
        })?;
        if file_type.is_symlink() {
            return Err(format!(
                "OpenCode session tree must not contain a symlink: {}",
                path.display()
            ));
        }
        if file_type.is_dir() {
            if let Some(found) = find_session_file_in_root(root, &path, expected)? {
                return Ok(Some(found));
            }
        } else if file_type.is_file()
            && path.file_name().and_then(|name| name.to_str()) == Some(expected)
        {
            let canonical = path.canonicalize().map_err(|e| {
                format!(
                    "Failed to canonicalize OpenCode session file {}: {e}",
                    path.display()
                )
            })?;
            if !canonical.starts_with(root) {
                return Err(format!(
                    "OpenCode session file is outside its storage root: {}",
                    path.display()
                ));
            }
            return Ok(Some(canonical));
        }
    }
    Ok(None)
}

fn remove_file_if_exists(path: &Path) -> std::io::Result<()> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err),
    }
}

fn remove_dir_all_if_exists(path: &Path) -> std::io::Result<()> {
    match std::fs::remove_dir_all(path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;
    use std::sync::{Mutex, OnceLock};
    use tempfile::tempdir;

    fn opencode_env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn create_sqlite_schema(conn: &Connection) {
        conn.execute_batch(
            "
            PRAGMA foreign_keys = ON;
            CREATE TABLE session (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                directory TEXT NOT NULL,
                time_created INTEGER NOT NULL,
                time_updated INTEGER NOT NULL
            );
            CREATE TABLE message (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                time_created INTEGER NOT NULL,
                data TEXT NOT NULL,
                FOREIGN KEY(session_id) REFERENCES session(id) ON DELETE CASCADE
            );
            CREATE TABLE part (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                message_id TEXT NOT NULL,
                time_created INTEGER NOT NULL,
                data TEXT NOT NULL,
                FOREIGN KEY(session_id) REFERENCES session(id) ON DELETE CASCADE,
                FOREIGN KEY(message_id) REFERENCES message(id) ON DELETE CASCADE
            );
            ",
        )
        .expect("create sqlite schema");
    }

    fn write_file_session_message(
        storage: &Path,
        session_id: &str,
        filename: &str,
        message_id: &str,
    ) -> PathBuf {
        let message_dir = storage.join("message").join(session_id);
        std::fs::create_dir_all(&message_dir).expect("create message dir");
        std::fs::write(
            message_dir.join(filename),
            serde_json::json!({ "id": message_id, "role": "user" }).to_string(),
        )
        .expect("write message");
        message_dir
    }

    #[test]
    fn delete_session_removes_session_diff_messages_and_parts() {
        let temp = tempdir().expect("tempdir");
        let storage = temp.path();
        let project_id = "project-123";
        let session_id = "ses_123";
        let session_dir = storage.join("session").join(project_id);
        let message_dir = storage.join("message").join(session_id);
        let session_diff = storage
            .join("session_diff")
            .join(format!("{session_id}.json"));
        let part_dir = storage.join("part").join("msg_1");
        let session_file = session_dir.join(format!("{session_id}.json"));

        std::fs::create_dir_all(&session_dir).expect("create session dir");
        std::fs::create_dir_all(&message_dir).expect("create message dir");
        std::fs::create_dir_all(&part_dir).expect("create part dir");
        std::fs::create_dir_all(storage.join("project")).expect("create project dir");
        std::fs::create_dir_all(storage.join("session_diff")).expect("create session diff dir");

        std::fs::write(
            &session_file,
            format!(
                r#"{{
                  "id": "{session_id}",
                  "projectID": "{project_id}",
                  "directory": "/tmp/project",
                  "time": {{ "created": 1, "updated": 2 }}
                }}"#
            ),
        )
        .expect("write session file");
        std::fs::write(
            message_dir.join("msg_1.json"),
            format!(r#"{{"id":"msg_1","sessionID":"{session_id}","role":"user"}}"#),
        )
        .expect("write message file");
        std::fs::write(
            part_dir.join("prt_1.json"),
            r#"{"id":"prt_1","messageID":"msg_1"}"#,
        )
        .expect("write part file");
        std::fs::write(&session_diff, "[]").expect("write session diff");
        std::fs::write(
            storage.join("project").join(format!("{project_id}.json")),
            r#"{"id":"project-123"}"#,
        )
        .expect("write project file");

        delete_session(storage, &message_dir, session_id).expect("delete session");

        assert!(!session_file.exists());
        assert!(!message_dir.exists());
        assert!(!session_diff.exists());
        assert!(!part_dir.exists());
        assert!(storage
            .join("project")
            .join(format!("{project_id}.json"))
            .exists());
    }

    #[test]
    fn delete_session_rejects_absolute_message_id_before_any_deletion() {
        let temp = tempdir().expect("tempdir");
        let storage = temp.path().join("storage");
        let session_id = "ses_absolute_escape";
        let outside = temp.path().join("outside-absolute");
        std::fs::create_dir_all(&outside).expect("create outside dir");
        let sentinel = outside.join("sentinel.txt");
        std::fs::write(&sentinel, "keep").expect("write sentinel");
        let message_dir = write_file_session_message(
            &storage,
            session_id,
            "message.json",
            outside.to_str().expect("utf-8 path"),
        );

        let result = delete_session(&storage, &message_dir, session_id);
        assert!(
            result.is_err(),
            "absolute message id was accepted: {result:?}; outside sentinel exists: {}",
            sentinel.exists()
        );
        let error = result.expect_err("checked error");

        assert!(error.contains("message ID"));
        assert!(sentinel.exists());
        assert!(message_dir.exists());
    }

    #[test]
    fn delete_session_rejects_parent_traversal_before_any_deletion() {
        let temp = tempdir().expect("tempdir");
        let storage = temp.path().join("storage");
        let session_id = "ses_parent_escape";
        std::fs::create_dir_all(storage.join("part")).expect("create part root");
        let outside = temp.path().join("outside-parent");
        std::fs::create_dir_all(&outside).expect("create outside dir");
        let sentinel = outside.join("sentinel.txt");
        std::fs::write(&sentinel, "keep").expect("write sentinel");
        let message_dir = write_file_session_message(
            &storage,
            session_id,
            "message.json",
            "../../outside-parent",
        );

        let result = delete_session(&storage, &message_dir, session_id);
        assert!(
            result.is_err(),
            "parent traversal was accepted: {result:?}; outside sentinel exists: {}",
            sentinel.exists()
        );
        let error = result.expect_err("checked error");

        assert!(error.contains("message ID"));
        assert!(sentinel.exists());
        assert!(message_dir.exists());
    }

    #[cfg(unix)]
    #[test]
    fn delete_session_rejects_symlink_part_target_before_any_deletion() {
        use std::os::unix::fs::symlink;

        let temp = tempdir().expect("tempdir");
        let storage = temp.path().join("storage");
        let session_id = "ses_symlink_escape";
        let part_root = storage.join("part");
        std::fs::create_dir_all(&part_root).expect("create part root");
        let outside = temp.path().join("outside-symlink");
        std::fs::create_dir_all(&outside).expect("create outside dir");
        let sentinel = outside.join("sentinel.txt");
        std::fs::write(&sentinel, "keep").expect("write sentinel");
        symlink(&outside, part_root.join("msg_symlink")).expect("create part symlink");
        let message_dir =
            write_file_session_message(&storage, session_id, "message.json", "msg_symlink");

        let result = delete_session(&storage, &message_dir, session_id);
        assert!(
            result.is_err(),
            "symlink part target was accepted: {result:?}; outside sentinel exists: {}",
            sentinel.exists()
        );
        let error = result.expect_err("checked error");

        assert!(error.contains("symlink"));
        assert!(sentinel.exists());
        assert!(message_dir.exists());
    }

    #[cfg(unix)]
    #[test]
    fn delete_session_rejects_symlink_part_root_before_any_deletion() {
        use std::os::unix::fs::symlink;

        let temp = tempdir().expect("tempdir");
        let storage = temp.path().join("storage");
        let session_id = "ses_part_root_symlink";
        std::fs::create_dir_all(&storage).expect("create storage");
        let outside = temp.path().join("outside-part-root");
        let outside_part = outside.join("msg_external");
        std::fs::create_dir_all(&outside_part).expect("create outside part");
        let sentinel = outside_part.join("sentinel.txt");
        std::fs::write(&sentinel, "keep").expect("write sentinel");
        symlink(&outside, storage.join("part")).expect("create part-root symlink");
        let message_dir =
            write_file_session_message(&storage, session_id, "message.json", "msg_external");

        let result = delete_session(&storage, &message_dir, session_id);
        assert!(
            result.is_err(),
            "symlink part root was accepted: {result:?}; outside sentinel exists: {}",
            sentinel.exists()
        );
        let error = result.expect_err("checked error");

        assert!(error.contains("symlink"));
        assert!(sentinel.exists());
        assert!(message_dir.exists());
    }

    #[test]
    fn delete_session_prevalidates_all_message_ids_atomically() {
        let temp = tempdir().expect("tempdir");
        let storage = temp.path().join("storage");
        let session_id = "ses_atomic_validation";
        let good_part = storage.join("part").join("msg_good");
        std::fs::create_dir_all(&good_part).expect("create good part");
        let good_sentinel = good_part.join("sentinel.txt");
        std::fs::write(&good_sentinel, "keep").expect("write good sentinel");
        let outside = temp.path().join("outside-atomic");
        std::fs::create_dir_all(&outside).expect("create outside dir");
        let outside_sentinel = outside.join("sentinel.txt");
        std::fs::write(&outside_sentinel, "keep").expect("write outside sentinel");
        let message_dir =
            write_file_session_message(&storage, session_id, "00-good.json", "msg_good");
        write_file_session_message(
            &storage,
            session_id,
            "99-malicious.json",
            outside.to_str().expect("utf-8 path"),
        );
        let session_diff = storage
            .join("session_diff")
            .join(format!("{session_id}.json"));
        std::fs::create_dir_all(session_diff.parent().expect("diff parent"))
            .expect("create diff dir");
        std::fs::write(&session_diff, "[]").expect("write session diff");

        let result = delete_session(&storage, &message_dir, session_id);
        assert!(
            result.is_err(),
            "mixed valid/malicious IDs were accepted: {result:?}; good sentinel exists: {}; outside sentinel exists: {}",
            good_sentinel.exists(),
            outside_sentinel.exists()
        );

        assert!(good_sentinel.exists());
        assert!(outside_sentinel.exists());
        assert!(message_dir.exists());
        assert!(session_diff.exists());
    }

    #[test]
    fn load_messages_includes_tool_parts() {
        let temp = tempdir().expect("tempdir");
        let storage = temp.path();
        let session_id = "ses_test";
        let msg_id = "msg_1";

        let msg_dir = storage.join("message").join(session_id);
        let part_dir = storage.join("part").join(msg_id);
        std::fs::create_dir_all(&msg_dir).expect("create msg dir");
        std::fs::create_dir_all(&part_dir).expect("create part dir");

        std::fs::write(
            msg_dir.join(format!("{msg_id}.json")),
            r#"{"id":"msg_1","role":"assistant","time":{"created":"2026-03-06T10:00:00Z"}}"#,
        )
        .expect("write msg");

        std::fs::write(
            part_dir.join("prt_1.json"),
            r#"{"id":"prt_1","type":"tool","tool":"bash","state":{"status":"completed","input":{"command":"ls"},"output":"file.txt"}}"#,
        )
        .expect("write tool part");

        std::fs::write(
            part_dir.join("prt_2.json"),
            r#"{"id":"prt_2","type":"text","text":"Here are the files."}"#,
        )
        .expect("write text part");

        let msgs = load_messages(&msg_dir).expect("load");
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].role, "assistant");
        assert!(msgs[0].content.contains("[Tool: bash]"));
        assert!(msgs[0].content.contains("Here are the files."));
    }

    #[test]
    fn parse_json_session_does_not_follow_unsafe_session_or_message_ids() {
        let temp = tempdir().expect("tempdir");
        let storage = temp.path().join("storage");
        let session_dir = storage.join("session").join("project");
        std::fs::create_dir_all(&session_dir).expect("create session dir");

        let outside_messages = temp.path().join("outside-messages");
        let outside_parts = temp.path().join("outside-parts");
        std::fs::create_dir_all(&outside_messages).expect("create outside messages");
        std::fs::create_dir_all(&outside_parts).expect("create outside parts");
        std::fs::write(
            outside_messages.join("message.json"),
            serde_json::json!({
                "id": outside_parts.to_str().expect("utf-8 path"),
                "role": "user",
                "time": { "created": 1 }
            })
            .to_string(),
        )
        .expect("write outside message");
        std::fs::write(
            outside_parts.join("part.json"),
            r#"{"type":"text","text":"must-not-leak"}"#,
        )
        .expect("write outside part");

        let session_path = session_dir.join("unsafe.json");
        std::fs::write(
            &session_path,
            serde_json::json!({
                "id": outside_messages.to_str().expect("utf-8 path"),
                "time": { "created": 1 }
            })
            .to_string(),
        )
        .expect("write session");

        let meta = parse_session(&storage, &session_path).expect("parse session metadata");
        assert_eq!(meta.source_path, None);
        assert_eq!(meta.summary, None);
        assert_eq!(meta.resume_command, None);
    }

    #[test]
    fn load_messages_rejects_unsafe_message_id_before_reading_parts() {
        let temp = tempdir().expect("tempdir");
        let storage = temp.path().join("storage");
        let message_dir = storage.join("message").join("ses_safe");
        let outside_parts = temp.path().join("outside-parts");
        std::fs::create_dir_all(&message_dir).expect("create message dir");
        std::fs::create_dir_all(&outside_parts).expect("create outside parts");
        std::fs::write(
            message_dir.join("message.json"),
            serde_json::json!({
                "id": outside_parts.to_str().expect("utf-8 path"),
                "role": "user"
            })
            .to_string(),
        )
        .expect("write message");
        std::fs::write(
            outside_parts.join("part.json"),
            r#"{"type":"text","text":"must-not-leak"}"#,
        )
        .expect("write outside part");

        let error = load_messages(&message_dir).expect_err("unsafe message id must fail closed");
        assert!(error.contains("message ID"));
        assert!(!error.contains("must-not-leak"));
    }

    #[cfg(unix)]
    #[test]
    fn load_messages_rejects_symlink_part_directory() {
        use std::os::unix::fs::symlink;

        let temp = tempdir().expect("tempdir");
        let storage = temp.path().join("storage");
        let message_dir = storage.join("message").join("ses_safe");
        let part_root = storage.join("part");
        let outside_parts = temp.path().join("outside-parts");
        std::fs::create_dir_all(&message_dir).expect("create message dir");
        std::fs::create_dir_all(&part_root).expect("create part root");
        std::fs::create_dir_all(&outside_parts).expect("create outside parts");
        std::fs::write(
            message_dir.join("message.json"),
            r#"{"id":"msg_safe","role":"user"}"#,
        )
        .expect("write message");
        std::fs::write(
            outside_parts.join("part.json"),
            r#"{"type":"text","text":"must-not-leak"}"#,
        )
        .expect("write outside part");
        symlink(&outside_parts, part_root.join("msg_safe")).expect("create part symlink");

        let error =
            load_messages(&message_dir).expect_err("symlink part directory must fail closed");
        assert!(error.contains("symlink"));
        assert!(!error.contains("must-not-leak"));
    }

    #[test]
    fn parse_sqlite_source_accepts_valid_references() {
        let parsed = parse_sqlite_source("sqlite:/tmp/opencode.db:ses_123").expect("valid source");

        assert_eq!(parsed.0, PathBuf::from("/tmp/opencode.db"));
        assert_eq!(parsed.1, "ses_123");
    }

    #[test]
    fn parse_sqlite_source_rejects_invalid_references() {
        assert!(parse_sqlite_source("/tmp/opencode.db:ses_123").is_none());
        assert!(parse_sqlite_source("sqlite:/tmp/opencode.db:msg_123").is_none());
        assert!(parse_sqlite_source("sqlite:/tmp/opencode.db").is_none());
    }

    #[test]
    #[allow(deprecated)] // set_var/remove_var deprecated since Rust 1.81; safe here under mutex
    fn scan_sessions_sqlite_reads_temp_database() {
        let _guard = opencode_env_lock().lock().expect("lock");
        let temp = tempdir().expect("tempdir");
        let original_xdg = std::env::var_os("XDG_DATA_HOME");
        std::env::set_var("XDG_DATA_HOME", temp.path());

        let base_dir = temp.path().join("opencode");
        std::fs::create_dir_all(&base_dir).expect("create base dir");
        let db_path = base_dir.join("opencode.db");
        let conn = Connection::open(&db_path).expect("open sqlite db");
        create_sqlite_schema(&conn);

        conn.execute(
            "INSERT INTO session (id, title, directory, time_created, time_updated) VALUES (?1, ?2, ?3, ?4, ?5)",
            ("ses_1", "", "/tmp/project-a", 1_771_061_953_033_i64, 1_771_061_954_033_i64),
        )
        .expect("insert session 1");
        conn.execute(
            "INSERT INTO session (id, title, directory, time_created, time_updated) VALUES (?1, ?2, ?3, ?4, ?5)",
            ("ses_2", "Named Session", "/tmp/project-b", 1_771_061_950_000_i64, 1_771_061_955_000_i64),
        )
        .expect("insert session 2");
        drop(conn);

        let sessions = scan_sessions_sqlite();

        #[allow(deprecated)]
        if let Some(value) = original_xdg {
            std::env::set_var("XDG_DATA_HOME", value);
        } else {
            std::env::remove_var("XDG_DATA_HOME");
        }

        assert_eq!(sessions.len(), 2);
        assert_eq!(sessions[0].session_id, "ses_2");
        assert_eq!(sessions[0].title.as_deref(), Some("Named Session"));
        assert_eq!(sessions[1].session_id, "ses_1");
        assert_eq!(sessions[1].title.as_deref(), Some("project-a"));
        assert_eq!(sessions[1].project_dir.as_deref(), Some("/tmp/project-a"));
        let expected_source = format!("sqlite:{}:ses_1", db_path.display());
        assert_eq!(
            sessions[1].source_path.as_deref(),
            Some(expected_source.as_str())
        );
        assert_eq!(
            sessions[1].resume_command.as_deref(),
            Some("opencode -s ses_1")
        );
    }

    #[test]
    #[allow(deprecated)] // set_var/remove_var deprecated since Rust 1.81; safe here under mutex
    fn scan_sessions_sqlite_suppresses_unsafe_resume_commands() {
        let _guard = opencode_env_lock().lock().expect("lock");
        let temp = tempdir().expect("tempdir");
        let original_xdg = std::env::var_os("XDG_DATA_HOME");
        std::env::set_var("XDG_DATA_HOME", temp.path());

        let base_dir = temp.path().join("opencode");
        std::fs::create_dir_all(&base_dir).expect("create base dir");
        let db_path = base_dir.join("opencode.db");
        let conn = Connection::open(&db_path).expect("open sqlite db");
        create_sqlite_schema(&conn);
        for (index, session_id) in [
            "x; /usr/bin/touch /tmp/fyagent-opencode-sqlite-injection #",
            "session & calc.exe & rem",
            "--dangerously-bypass-approvals-and-sandbox",
        ]
        .into_iter()
        .enumerate()
        {
            conn.execute(
                "INSERT INTO session (id, title, directory, time_created, time_updated) VALUES (?1, ?2, ?3, ?4, ?5)",
                (session_id, "Hostile ID", "/tmp/project", 1000_i64, 3000_i64 + index as i64),
            )
            .expect("insert session");
        }
        drop(conn);

        let sessions = scan_sessions_sqlite();

        if let Some(value) = original_xdg {
            std::env::set_var("XDG_DATA_HOME", value);
        } else {
            std::env::remove_var("XDG_DATA_HOME");
        }

        assert_eq!(sessions.len(), 3);
        assert!(sessions
            .iter()
            .all(|session| session.resume_command.is_none()));
    }

    #[test]
    fn parse_json_session_preserves_plain_and_suppresses_unsafe_resume_commands() {
        let temp = tempdir().expect("tempdir");
        let storage = temp.path();

        let plain_path = storage.join("plain.json");
        std::fs::write(
            &plain_path,
            serde_json::json!({ "id": "ses_plain-1", "title": "Plain" }).to_string(),
        )
        .expect("write plain session");
        let plain = parse_session(storage, &plain_path).expect("parse plain session");
        assert_eq!(
            plain.resume_command.as_deref(),
            Some("opencode -s ses_plain-1")
        );

        for (index, session_id) in [
            "line-one\n\"line-two\"",
            "session & calc.exe & rem",
            "--dangerously-bypass-approvals-and-sandbox",
        ]
        .into_iter()
        .enumerate()
        {
            let hostile_path = storage.join(format!("hostile-{index}.json"));
            std::fs::write(
                &hostile_path,
                serde_json::json!({ "id": session_id, "title": "Hostile" }).to_string(),
            )
            .expect("write hostile session");
            let hostile = parse_session(storage, &hostile_path).expect("parse hostile session");
            assert_eq!(hostile.resume_command, None, "unsafe id: {session_id}");
        }
    }

    #[test]
    fn load_messages_sqlite_reads_messages_and_parts() {
        let temp = tempdir().expect("tempdir");
        let db_path = temp.path().join("opencode.db");
        let conn = Connection::open(&db_path).expect("open sqlite db");
        create_sqlite_schema(&conn);

        conn.execute(
            "INSERT INTO session (id, title, directory, time_created, time_updated) VALUES (?1, ?2, ?3, ?4, ?5)",
            ("ses_1", "Session", "/tmp/project-a", 1000_i64, 3000_i64),
        )
        .expect("insert session");
        conn.execute(
            "INSERT INTO message (id, session_id, time_created, data) VALUES (?1, ?2, ?3, ?4)",
            ("msg_1", "ses_1", 1000_i64, r#"{"role":"user"}"#),
        )
        .expect("insert message 1");
        conn.execute(
            "INSERT INTO message (id, session_id, time_created, data) VALUES (?1, ?2, ?3, ?4)",
            ("msg_2", "ses_1", 2000_i64, r#"{"role":"assistant"}"#),
        )
        .expect("insert message 2");
        conn.execute(
            "INSERT INTO part (id, session_id, message_id, time_created, data) VALUES (?1, ?2, ?3, ?4, ?5)",
            ("prt_1", "ses_1", "msg_1", 1000_i64, r#"{"type":"text","text":"Hello"}"#),
        )
        .expect("insert part 1");
        conn.execute(
            "INSERT INTO part (id, session_id, message_id, time_created, data) VALUES (?1, ?2, ?3, ?4, ?5)",
            (
                "prt_2",
                "ses_1",
                "msg_2",
                2000_i64,
                r#"{"type":"tool","tool":"bash"}"#,
            ),
        )
        .expect("insert part 2");
        conn.execute(
            "INSERT INTO part (id, session_id, message_id, time_created, data) VALUES (?1, ?2, ?3, ?4, ?5)",
            (
                "prt_3",
                "ses_1",
                "msg_2",
                2001_i64,
                r#"{"type":"text","text":"Done"}"#,
            ),
        )
        .expect("insert part 3");
        drop(conn);

        let source = format!("sqlite:{}:ses_1", db_path.display());
        let messages = load_messages_sqlite(&source).expect("load sqlite messages");

        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, "user");
        assert_eq!(messages[0].content, "Hello");
        assert_eq!(messages[0].ts, Some(1000));
        assert_eq!(messages[1].role, "assistant");
        assert_eq!(messages[1].content, "[Tool: bash]\nDone");
        assert_eq!(messages[1].ts, Some(2000));
    }

    #[test]
    fn delete_session_sqlite_removes_session() {
        let _guard = opencode_env_lock().lock().expect("lock");
        let temp = tempdir().expect("tempdir");
        let original_xdg = std::env::var_os("XDG_DATA_HOME");
        #[allow(deprecated)]
        std::env::set_var("XDG_DATA_HOME", temp.path());

        let base_dir = temp.path().join("opencode");
        std::fs::create_dir_all(&base_dir).expect("create base dir");
        let db_path = base_dir.join("opencode.db");
        let conn = Connection::open(&db_path).expect("open sqlite db");
        create_sqlite_schema(&conn);

        conn.execute(
            "INSERT INTO session (id, title, directory, time_created, time_updated) VALUES (?1, ?2, ?3, ?4, ?5)",
            ("ses_1", "Session", "/tmp/project-a", 1000_i64, 3000_i64),
        )
        .expect("insert session");
        conn.execute(
            "INSERT INTO message (id, session_id, time_created, data) VALUES (?1, ?2, ?3, ?4)",
            ("msg_1", "ses_1", 1000_i64, r#"{"role":"user"}"#),
        )
        .expect("insert message");
        conn.execute(
            "INSERT INTO part (id, session_id, message_id, time_created, data) VALUES (?1, ?2, ?3, ?4, ?5)",
            ("prt_1", "ses_1", "msg_1", 1000_i64, r#"{"type":"text","text":"Hello"}"#),
        )
        .expect("insert part");
        drop(conn);

        let source = format!("sqlite:{}:ses_1", db_path.display());
        let deleted = delete_session_sqlite("ses_1", &source).expect("delete sqlite session");
        assert!(deleted);

        let conn = Connection::open(&db_path).expect("re-open sqlite db");
        let remaining_sessions: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM session WHERE id = 'ses_1'",
                [],
                |row| row.get(0),
            )
            .expect("count sessions");
        let remaining_messages: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM message WHERE session_id = 'ses_1'",
                [],
                |row| row.get(0),
            )
            .expect("count messages");
        let remaining_parts: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM part WHERE session_id = 'ses_1'",
                [],
                |row| row.get(0),
            )
            .expect("count parts");

        assert_eq!(remaining_sessions, 0);
        assert_eq!(remaining_messages, 0);
        assert_eq!(remaining_parts, 0);

        #[allow(deprecated)]
        if let Some(value) = original_xdg {
            std::env::set_var("XDG_DATA_HOME", value);
        } else {
            std::env::remove_var("XDG_DATA_HOME");
        }
    }

    #[test]
    fn delete_session_sqlite_rejects_foreign_db_path() {
        let _guard = opencode_env_lock().lock().expect("lock");
        let temp = tempdir().expect("tempdir");
        let original_xdg = std::env::var_os("XDG_DATA_HOME");
        #[allow(deprecated)]
        std::env::set_var("XDG_DATA_HOME", temp.path());

        let expected_base_dir = temp.path().join("opencode");
        std::fs::create_dir_all(&expected_base_dir).expect("create expected base dir");
        let expected_db_path = expected_base_dir.join("opencode.db");
        Connection::open(&expected_db_path).expect("create expected sqlite db");

        let db_path = temp.path().join("foreign.db");
        let conn = Connection::open(&db_path).expect("open sqlite db");
        create_sqlite_schema(&conn);
        conn.execute(
            "INSERT INTO session (id, title, directory, time_created, time_updated) VALUES (?1, ?2, ?3, ?4, ?5)",
            ("ses_1", "Session", "/tmp/project", 1000_i64, 3000_i64),
        )
        .expect("insert session");
        drop(conn);

        let source = format!("sqlite:{}:ses_1", db_path.display());
        let err = delete_session_sqlite("ses_1", &source).expect_err("should reject foreign db");
        assert!(err.contains("expected OpenCode database"));

        #[allow(deprecated)]
        if let Some(value) = original_xdg {
            std::env::set_var("XDG_DATA_HOME", value);
        } else {
            std::env::remove_var("XDG_DATA_HOME");
        }
    }
}
