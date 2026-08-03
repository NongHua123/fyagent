//! WorkBuddy's isolated configuration domain.
//!
//! This module has no Provider/AppType input and owns the only three IPC
//! operations needed by the WorkBuddy page.

pub mod config;
pub mod error;
pub mod model_fetch;
pub mod types;
pub mod url;

pub(crate) use config::{get_workbuddy_status, save_workbuddy_models};
pub(crate) use model_fetch::fetch_workbuddy_models;
