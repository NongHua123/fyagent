//! Pure installer domain logic.
//!
//! Tauri commands and `AppState` wiring deliberately live outside this module;
//! integration owns that shared registration boundary.

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
