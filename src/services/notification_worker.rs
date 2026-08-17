//! 通知投递 Worker（Kafka 消费者）。
//!
//! 消费 notifications topic，按用户在线状态分流投递：
//! - 在线（WS hub 有连接）→ WebSocket 推送 notice + unread 同步
//! - 离线 → 系统推送（Bark 等）
//!
//! 语义：at-least-once。手动 commit offset；
//! 单条投递失败在进程内退避重试（1s/2s/4s），重试耗尽记录日志后
//! 仍 commit 跳过，避免毒消息阻塞分区。

use std::sync::Arc;
use std::time::Duration;

use rdkafka::config::ClientConfig;
use rdkafka::consumer::{CommitMode, Consumer, StreamConsumer};
use rdkafka::message::{BorrowedMessage, Message};
use serde::{Deserialize, Serialize};

use crate::config::KafkaConfig;
use crate::domain::push::PushToken;
use crate::error::AppError;
use crate::services::notice_store::NoticeStore;
use crate::services::push::{PushMessage, PushProvider, PushTokenStore};
use crate::services::ws_hub::WsHub;

/// 单条消息投递失败后的进程内重试次数（总尝试 = 重试 + 1）
const MAX_RETRIES: u32 = 3;

#[derive(Deserialize, Serialize)]
pub struct NoticePayload {
    pub id: String,
    pub streamer_id: String,
    pub streamer_nickname: String,
    pub avatar: Option<String>,
    pub title: String,
    pub body: String,
    pub live_started_at: Option<i64>,
    pub created_at: i64,
}

pub struct NotificationWorker {
    consumer: StreamConsumer,
    notice_store: Arc<dyn NoticeStore>,
    push_token_store: Arc<dyn PushTokenStore>,
    push_providers: Vec<Arc<dyn PushProvider>>,
    ws_hub: Arc<WsHub>,
}

impl NotificationWorker {
    pub fn new(
        config: &KafkaConfig,
        notice_store: Arc<dyn NoticeStore>,
        push_token_store: Arc<dyn PushTokenStore>,
        push_providers: Vec<Arc<dyn PushProvider>>,
        ws_hub: Arc<WsHub>,
    ) -> Result<Self, AppError> {
        let consumer: StreamConsumer = ClientConfig::new()
            .set("bootstrap.servers", &config.brokers)
            .set("group.id", &config.group_id)
            // 手动 commit：投递完成后才确认 offset
            .set("enable.auto.commit", "false")
            // 消费组首次启动从头消费，保证 at-least-once
            .set("auto.offset.reset", "earliest")
            .create()
            .map_err(|e| AppError::internal(format!("Kafka consumer 创建失败: {e}")))?;
        consumer
            .subscribe(&[&config.notification_topic])
            .map_err(|e| AppError::internal(format!("Kafka 订阅 topic 失败: {e}")))?;
        Ok(Self {
            consumer,
            notice_store,
            push_token_store,
            push_providers,
            ws_hub,
        })
    }

    /// 常驻消费循环
    ///
    /// 连接级错误（broker 不可用等）按指数退避重试，避免 Kafka 故障时刷屏日志。
    pub async fn run(self: Arc<Self>) {
        tracing::info!("通知投递 Worker 启动");
        let mut backoff_secs = 1u64;
        loop {
            match self.consumer.recv().await {
                Ok(message) => {
                    backoff_secs = 1; // 恢复后重置退避
                    if let Err(error) = self.handle_message(&message).await {
                        // 解析失败或重试耗尽：记录后跳过（毒消息不阻塞分区）
                        tracing::error!(?error, "通知投递最终失败，已跳过");
                    }
                    if let Err(error) = self.consumer.commit_message(&message, CommitMode::Async) {
                        tracing::warn!(?error, "Kafka offset commit 失败");
                    }
                }
                Err(error) => {
                    tracing::error!(?error, retry_in_secs = backoff_secs, "Kafka 消费异常");
                    tokio::time::sleep(Duration::from_secs(backoff_secs)).await;
                    backoff_secs = (backoff_secs * 2).min(30);
                }
            }
        }
    }

    /// 解析并投递单条消息，失败时退避重试
    async fn handle_message(&self, message: &BorrowedMessage<'_>) -> Result<(), AppError> {
        let user_id = std::str::from_utf8(message.key().unwrap_or_default())
            .map_err(|_| AppError::internal("通知消息 key 非法"))?
            .to_string();
        let payload = message
            .payload_view::<str>()
            .transpose()
            .map_err(|_| AppError::internal("通知消息 payload 非法"))?
            .ok_or_else(|| AppError::internal("通知消息 payload 为空"))?;
        let notice: NoticePayload = serde_json::from_str(payload)
            .map_err(|_| AppError::internal("通知消息 payload 解析失败"))?;

        for attempt in 0..=MAX_RETRIES {
            match self.deliver(&user_id, &notice).await {
                Ok(()) => return Ok(()),
                Err(error) if attempt < MAX_RETRIES => {
                    let delay = 1u64 << attempt; // 1s → 2s → 4s
                    tracing::warn!(user_id = %user_id, attempt, ?error, "通知投递失败，稍后重试");
                    tokio::time::sleep(Duration::from_secs(delay)).await;
                }
                Err(error) => return Err(error),
            }
        }
        unreachable!("重试循环必然 return")
    }

    /// 分流投递：在线走 WS，离线走系统推送
    async fn deliver(&self, user_id: &str, notice: &NoticePayload) -> Result<(), AppError> {
        let payload = serde_json::json!({
            "type": "notice",
            "data": notice,
        });
        if self.ws_hub.send_to_user(user_id, &payload.to_string()) {
            let unread = self.notice_store.unread_count(user_id).await?;
            let sync = serde_json::json!({"type": "unread", "data": {"count": unread}});
            self.ws_hub.send_to_user(user_id, &sync.to_string());
            return Ok(());
        }

        let tokens = self.push_token_store.list_by_user(user_id).await?;
        self.send_pushes(tokens, notice).await
    }

    async fn send_pushes(
        &self,
        tokens: Vec<PushToken>,
        notice: &NoticePayload,
    ) -> Result<(), AppError> {
        let message = PushMessage {
            title: notice.title.clone(),
            body: notice.body.clone(),
            url: None,
        };
        for token in tokens {
            let provider = self
                .push_providers
                .iter()
                .find(|provider| provider.channel() == token.channel)
                .ok_or_else(|| AppError::internal(format!("未配置推送通道 {}", token.channel)))?;
            provider.send(&token.token, &message).await?;
        }
        Ok(())
    }
}
