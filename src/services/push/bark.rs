//! Bark 推送通道实现
//!
//! Bark 是 iOS 上知名的自托管推送服务，用户安装 Bark App 后会得到一个 key，
//! 通过 HTTP POST 到 {base_url}/{key} 即可发送通知。
//! 此实现把 bark_base_url + 用户的 key 拼接成完整 URL。

use std::time::Duration;

use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;

use crate::error::AppError;
use crate::services::push::provider::{PushMessage, PushProvider};

pub struct BarkProvider {
    base_url: String,
    client: Client,
}

impl BarkProvider {
    pub fn new(base_url: String, timeout_secs: u64) -> Result<Self, AppError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .build()
            .map_err(|e| AppError::internal(format!("Bark HTTP client 初始化失败: {e}")))?;
        Ok(Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            client,
        })
    }
}

#[async_trait]
impl PushProvider for BarkProvider {
    fn channel(&self) -> &'static str {
        "bark"
    }

    async fn send(&self, token: &str, msg: &PushMessage) -> Result<(), AppError> {
        let url = format!("{}/{}", self.base_url, token);

        let mut body = json!({
            "title": msg.title,
            "body": msg.body,
        });
        if let Some(u) = &msg.url {
            body["url"] = json!(u);
        }

        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| AppError::internal(format!("Bark 推送请求失败: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(AppError::internal(format!(
                "Bark 推送失败 status={status} body={text}"
            )));
        }
        Ok(())
    }
}
