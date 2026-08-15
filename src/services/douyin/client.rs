//! 抖音 enter 接口客户端
//!
//! 封装 ttwid 自动注册/刷新 + enter 接口请求 + JSON 解析。
//! [super::live_checker::HttpLiveChecker] 与
//! [super::streamer_resolver::HttpStreamerResolver] 共享同一实例（Arc）。

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::Mutex;

use crate::config::DouyinConfig;
use crate::error::AppError;

use super::enter_parser::{parse, EnterRoomData};

/// 抖音 enter 接口客户端
pub struct DouyinEnterClient {
    client: reqwest::Client,
    ttwid: Mutex<Option<String>>,
    config: Arc<DouyinConfig>,
}

impl DouyinEnterClient {
    pub fn new(config: Arc<DouyinConfig>) -> Result<Self, AppError> {
        let client = reqwest::Client::builder()
            .default_headers(Self::browser_headers(&config))
            .redirect(reqwest::redirect::Policy::limited(config.max_redirects))
            .timeout(Duration::from_secs(config.http_timeout_secs))
            .build()
            .map_err(|e| AppError::internal(format!("构建 reqwest client 失败: {e}")))?;
        Ok(Self {
            client,
            ttwid: Mutex::new(None),
            config,
        })
    }

    fn browser_headers(config: &DouyinConfig) -> reqwest::header::HeaderMap {
        use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, ACCEPT_LANGUAGE, REFERER, USER_AGENT};
        let mut h = HeaderMap::new();
        if let Ok(v) = HeaderValue::from_str(&config.user_agent) {
            h.insert(USER_AGENT, v);
        }
        if let Ok(v) = HeaderValue::from_str(&config.referer) {
            h.insert(REFERER, v);
        }
        h.insert(ACCEPT, HeaderValue::from_static("application/json, text/plain, */*"));
        h.insert(ACCEPT_LANGUAGE, HeaderValue::from_static("zh-CN,zh;q=0.9"));
        h
    }

    /// 获取 ttwid：优先用缓存，缓存为空则注册
    async fn get_ttwid(&self) -> Result<String, AppError> {
        {
            let cache = self.ttwid.lock().await;
            if let Some(t) = cache.as_ref() {
                return Ok(t.clone());
            }
        }
        self.refresh_ttwid().await
    }

    /// 强制重新注册 ttwid 并写入缓存
    async fn refresh_ttwid(&self) -> Result<String, AppError> {
        let ttwid = self.register_ttwid().await?;
        let mut cache = self.ttwid.lock().await;
        *cache = Some(ttwid.clone());
        Ok(ttwid)
    }

    /// 调用字节跳动 ttwid 注册接口
    async fn register_ttwid(&self) -> Result<String, AppError> {
        let body = serde_json::json!({
            "region": "cn",
            "aid": 1768,
            "needFid": false,
            "service": "www.ixigua.com",
            "migrate_info": { "ticket": "", "source": "node" },
            "cbUrlProtocol": "https",
            "union": true
        });
        let resp = self
            .client
            .post(&self.config.ttwid_register_url)
            .json(&body)
            .send()
            .await
            .map_err(|e| AppError::internal(format!("注册 ttwid 请求失败: {e}")))?;

        if !resp.status().is_success() {
            return Err(AppError::internal(format!(
                "注册 ttwid 接口返回非 200: {}",
                resp.status()
            )));
        }

        let ttwid = resp
            .cookies()
            .find(|c| c.name() == "ttwid")
            .map(|c| c.value().to_string())
            .ok_or_else(|| AppError::internal("注册 ttwid 响应中未找到 ttwid cookie"))?;
        tracing::info!("生成 ttwid 成功: {}...", &ttwid[..ttwid.len().min(16)]);
        Ok(ttwid)
    }

    /// 请求 enter 接口并解析，返回完整主播信息 + 开播状态
    ///
    /// 首次失败（多为 ttwid 过期）会自动刷新 ttwid 并重试一次。
    pub async fn enter(&self, douyin_id: &str) -> Result<EnterRoomData, AppError> {
        let ttwid = self.get_ttwid().await?;
        match self.query_enter(douyin_id, &ttwid).await {
            Ok(data) => Ok(data),
            Err(e) => {
                tracing::warn!("douyin enter query failed, refreshing ttwid and retrying: {:?}", e);
                let new_ttwid = self.refresh_ttwid().await?;
                self.query_enter(douyin_id, &new_ttwid).await
            }
        }
    }

    /// 单次请求 enter 接口
    async fn query_enter(
        &self,
        douyin_id: &str,
        ttwid: &str,
    ) -> Result<EnterRoomData, AppError> {
        let cookie = format!("ttwid={}", ttwid);

        let resp = self
            .client
            .get(&self.config.enter_api_url)
            .header(reqwest::header::COOKIE, &cookie)
            .query(&[
                ("aid", "6383"),
                ("device_platform", "web"),
                ("enter_from", "web_live"),
                ("cookie_enabled", "true"),
                ("browser_language", "zh-CN"),
                ("browser_platform", "Win32"),
                ("browser_name", "Chrome"),
                ("browser_version", "109.0.0.0"),
                ("web_rid", douyin_id),
            ])
            .send()
            .await
            .map_err(|e| AppError::internal(format!("请求抖音直播间接口失败: {e}")))?;

        if !resp.status().is_success() {
            return Err(AppError::internal(format!(
                "抖音直播间接口返回非 200: {}",
                resp.status()
            )));
        }
        let body = resp
            .text()
            .await
            .map_err(|e| AppError::internal(format!("读取响应失败: {e}")))?;
        if body.is_empty() {
            return Err(AppError::internal("抖音直播间接口响应内容为空"));
        }
        parse(&body)
    }
}
