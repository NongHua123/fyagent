//! Pure installer domain logic.
//!
//! Tauri commands and `AppState` wiring deliberately live outside this module;
//! integration owns that shared registration boundary.

// Every host parses the reserved flags before Tauri starts, but the
// provisioning protocol itself has a Windows-only runtime caller.
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub(crate) mod all_users;
pub mod cancellation;
pub mod download;
pub mod error;
pub mod jobs;
pub mod platform;
pub(crate) mod runtime;
pub mod source;
pub mod temp;
pub mod types;
pub mod verify;
