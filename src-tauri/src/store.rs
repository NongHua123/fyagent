use crate::codex_desktop_runtime::production_service;
use crate::database::Database;
use crate::services::{CodexDesktopService, ProxyService, UsageCache};
use std::sync::Arc;

/// 全局应用状态
pub struct AppState {
    pub db: Arc<Database>,
    pub proxy_service: ProxyService,
    pub usage_cache: Arc<UsageCache>,
    /// Process-local installer state. Its factory is inert: no metadata or
    /// package I/O is performed while constructing ordinary application state.
    pub codex_desktop_service: Arc<CodexDesktopService>,
}

impl AppState {
    /// 创建新的应用状态
    pub fn new(db: Arc<Database>) -> Self {
        let proxy_service = ProxyService::new(db.clone());

        Self {
            db,
            proxy_service,
            usage_cache: Arc::new(UsageCache::new()),
            codex_desktop_service: Arc::new(production_service()),
        }
    }
}
