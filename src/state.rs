//! 应用状态与依赖注入

use std::sync::Arc;

use crate::config::AppConfig;
use crate::services::code_store::{CodeStore, InMemoryCodeStore};
use crate::services::sms_provider::{MockSmsProvider, SmsProvider};

/// 应用共享状态，通过 axum `State` 注入到 handler。
///
/// 持有配置与所有基础设施实现（trait 对象），替换实现时只改 `build_state`。
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<AppConfig>,
    pub sms_provider: Arc<dyn SmsProvider>,
    pub code_store: Arc<dyn CodeStore>,
}

impl AppState {
    pub fn new(
        config: AppConfig,
        sms_provider: Arc<dyn SmsProvider>,
        code_store: Arc<dyn CodeStore>,
    ) -> Self {
        Self {
            config: Arc::new(config),
            sms_provider,
            code_store,
        }
    }
}

/// 组装默认 AppState（mock 实现）。
///
/// 切换为真实实现时，在此处替换 `sms_provider` / `code_store` 即可，
/// 业务层（service / handler）无需改动。
pub fn build_state(config: AppConfig) -> AppState {
    let sms_provider: Arc<dyn SmsProvider> = Arc::new(MockSmsProvider);
    let code_store: Arc<dyn CodeStore> = Arc::new(InMemoryCodeStore::new());
    AppState::new(config, sms_provider, code_store)
}
