//! 通知消息队列生产者（Kafka）。
//!
//! 调度器检测到开播后，将通知事件发布到 Kafka topic，
//! 由 `notification_worker` 异步消费并投递（WS/Bark）。
//!
//! 消息格式：key=user_id（同用户分区内有序），value=通知 JSON。

use std::time::Duration;

use async_trait::async_trait;
use rdkafka::config::ClientConfig;
use rdkafka::producer::{FutureProducer, FutureRecord};
use serde_json::Value;

use crate::config::KafkaConfig;
use crate::error::AppError;

/// 通知队列抽象（发布侧）
#[async_trait]
pub trait NotificationQueue: Send + Sync {
    /// 发布一条通知事件；key=user_id 保证同一用户的通知有序
    async fn publish(&self, user_id: &str, payload: Value) -> Result<(), AppError>;
}

pub struct KafkaNotificationQueue {
    producer: FutureProducer,
    topic: String,
}

impl KafkaNotificationQueue {
    pub fn new(config: &KafkaConfig) -> Result<Self, AppError> {
        let producer: FutureProducer = ClientConfig::new()
            .set("bootstrap.servers", &config.brokers)
            // broker 确认超时；开发单机足够
            .set("message.timeout.ms", "10000")
            .create()
            .map_err(|e| AppError::internal(format!("Kafka producer 创建失败: {e}")))?;
        Ok(Self {
            producer,
            topic: config.notification_topic.clone(),
        })
    }
}

#[async_trait]
impl NotificationQueue for KafkaNotificationQueue {
    async fn publish(&self, user_id: &str, payload: Value) -> Result<(), AppError> {
        let key = user_id.to_string();
        let body = payload.to_string();
        let record = FutureRecord::<String, String>::to(&self.topic)
            .key(&key)
            .payload(&body);
        self.producer
            .send(record, Duration::from_secs(10))
            .await
            .map(|(_, _)| ())
            .map_err(|(e, _)| AppError::internal(format!("Kafka 发布通知失败: {e}")))
    }
}
