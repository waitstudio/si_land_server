//! Outbox 异步投递 Worker。

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::domain::push::PushToken;
use crate::error::AppError;
use crate::services::notice_store::NoticeStore;
use crate::services::notification_outbox::{NotificationOutbox, OutboxEvent};
use crate::services::push::{PushMessage, PushProvider, PushTokenStore};
use crate::services::ws_hub::WsHub;
use crate::utils::time;

#[derive(Deserialize, Serialize)]
struct NoticePayload {
    id: String,
    streamer_id: String,
    streamer_nickname: String,
    avatar: Option<String>,
    title: String,
    body: String,
    live_started_at: Option<i64>,
    created_at: i64,
}

pub struct OutboxWorker {
    outbox: Arc<dyn NotificationOutbox>,
    notice_store: Arc<dyn NoticeStore>,
    push_token_store: Arc<dyn PushTokenStore>,
    push_providers: Vec<Arc<dyn PushProvider>>,
    ws_hub: Arc<WsHub>,
}

impl OutboxWorker {
    pub fn new(
        outbox: Arc<dyn NotificationOutbox>,
        notice_store: Arc<dyn NoticeStore>,
        push_token_store: Arc<dyn PushTokenStore>,
        push_providers: Vec<Arc<dyn PushProvider>>,
        ws_hub: Arc<WsHub>,
    ) -> Self {
        Self {
            outbox,
            notice_store,
            push_token_store,
            push_providers,
            ws_hub,
        }
    }

    pub async fn run(self: Arc<Self>) {
        let interval = std::time::Duration::from_secs(1);
        loop {
            if let Err(error) = self.tick().await {
                tracing::error!(?error, "outbox worker tick failed");
            }
            tokio::time::sleep(interval).await;
        }
    }

    async fn tick(&self) -> Result<(), AppError> {
        let events = self.outbox.claim_due(time::now_ts(), 100, 30).await?;
        for event in events {
            if let Err(error) = self.deliver(&event).await {
                let delay = 2_i64.pow((event.attempts.clamp(0, 7)) as u32).min(300);
                tracing::warn!(event_id = %event.id, ?error, "outbox delivery failed");
                self.outbox
                    .retry(&event.id, time::now_ts() + delay, &error.to_string())
                    .await?;
            } else {
                self.outbox.complete(&event.id).await?;
            }
        }
        Ok(())
    }

    async fn deliver(&self, event: &OutboxEvent) -> Result<(), AppError> {
        let notice: NoticePayload = serde_json::from_value(event.payload.clone())
            .map_err(|_| AppError::internal("无效的通知 Outbox 载荷"))?;
        let payload = serde_json::json!({
            "type": "notice",
            "data": notice,
        });
        if self
            .ws_hub
            .send_to_user(&event.user_id, &payload.to_string())
        {
            let unread = self.notice_store.unread_count(&event.user_id).await?;
            let sync = serde_json::json!({"type": "unread", "data": {"count": unread}});
            self.ws_hub.send_to_user(&event.user_id, &sync.to_string());
            return Ok(());
        }

        let tokens = self.push_token_store.list_by_user(&event.user_id).await?;
        self.send_pushes(tokens, &notice).await
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
