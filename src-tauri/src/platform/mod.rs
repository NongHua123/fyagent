//! Business-scoped platform integrations that must not become renderer-owned
//! command execution APIs.

pub(crate) mod process_launch;

#[cfg(target_os = "windows")]
pub(crate) mod windows;
