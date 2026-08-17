//! WebSocket 连接管理器（Hub）
//!
//! 维护 user_id → 多条连接（同一用户可能多设备同时在线）的映射。
//! 业务侧（如轮询调度器）通过 [WsHub::send_to_user] 向指定用户的所有
//! 在线连接推送消息；发送失败视为该用户不在线。
//!
//! 协议见 `api/v1/app/ws/mod.rs` 模块注释。

use std::collections::HashMap;
use std::sync::RwLock;

use axum::extract::ws::Message;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

/// 每用户允许的最大并发连接数（超出时关闭最旧的接收端）
const MAX_CONNECTIONS_PER_USER: usize = 5;

/// 连接管理器
///
/// 每条连接对应一对 (tx, rx)：tx 存于 Hub 供业务侧投递，rx 由会话 task
/// 消费转发到 socket。会话结束时调用 [WsHub::unregister] 精确注销。
#[derive(Default)]
pub struct WsHub {
    inner: RwLock<HashMap<String, Vec<UnboundedSender<Message>>>>,
}

impl WsHub {
    pub fn new() -> Self {
        Self::default()
    }

    /// 注册一条连接，返回 (tx, rx) 对
    ///
    /// tx 由调用方保存，会话结束时传给 [WsHub::unregister]；
    /// rx 由会话 task 持有并消费。
    pub fn register(
        &self,
        user_id: &str,
    ) -> (UnboundedSender<Message>, UnboundedReceiver<Message>) {
        let (tx, rx) = unbounded_channel();
        let mut map = self.inner.write().expect("WsHub 锁中毒");
        let list = map.entry(user_id.to_string()).or_default();
        // 超限时移除最旧连接：drop tx → 对应 rx.recv() 返回 None → 会话自动结束
        while list.len() >= MAX_CONNECTIONS_PER_USER {
            list.remove(0);
        }
        list.push(tx.clone());
        (tx, rx)
    }

    /// 注销一条连接（会话结束时调用，幂等）
    pub fn unregister(&self, user_id: &str, tx: &UnboundedSender<Message>) {
        let mut map = self.inner.write().expect("WsHub 锁中毒");
        if let Some(list) = map.get_mut(user_id) {
            list.retain(|t| !t.same_channel(tx));
            if list.is_empty() {
                map.remove(user_id);
            }
        }
    }

    /// 向指定用户的所有在线连接发送文本消息
    ///
    /// 返回是否至少有一条连接发送成功（即用户当前在线）。
    /// 发送失败的连接视为死连接，顺手清理。
    pub fn send_to_user(&self, user_id: &str, text: &str) -> bool {
        let mut map = self.inner.write().expect("WsHub 锁中毒");
        let Some(list) = map.get_mut(user_id) else {
            return false;
        };
        let msg = Message::Text(text.to_string().into());
        let mut delivered = false;
        list.retain(|tx| {
            if tx.send(msg.clone()).is_ok() {
                delivered = true;
                true
            } else {
                false
            }
        });
        if list.is_empty() {
            map.remove(user_id);
        }
        delivered
    }

    /// 判断用户是否有存活连接
    pub fn is_online(&self, user_id: &str) -> bool {
        self.inner
            .read()
            .expect("WsHub 锁中毒")
            .get(user_id)
            .is_some_and(|l| !l.is_empty())
    }

    /// 当前在线用户数（监控用）
    pub fn online_user_count(&self) -> usize {
        self.inner.read().expect("WsHub 锁中毒").len()
    }
}
