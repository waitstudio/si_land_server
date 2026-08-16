//! WebSocket 会话实现
//!
//! 会话循环（tokio::select! 两路复用）：
//! 1. Hub 接收端有消息 → 转发到 socket
//! 2. socket 收到客户端消息 → 处理心跳/关闭；90s 空闲则主动断开

use std::sync::Arc;
use std::time::Duration;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Query, State};
use axum::response::Response;
use futures_util::SinkExt;
use futures_util::stream::StreamExt;
use serde::Deserialize;
use serde_json::json;

use crate::error::AppError;
use crate::services::notice_store::NoticeStore;
use crate::services::ws_hub::WsHub;
use crate::state::AppState;

/// 客户端心跳间隔要求：90s 内无消息视为死连接
const IDLE_TIMEOUT: Duration = Duration::from_secs(90);

/// WS 握手 query 参数
#[derive(Debug, Deserialize)]
pub struct WsQuery {
    pub ticket: String,
}

/// GET /api/v1/app/ws?ticket=...
///
/// 先消费一次性 ticket 换取用户身份再升级协议；校验失败返回 401（非 WS 响应），
/// 客户端据此判定为鉴权失败并停止重连（需重新登录）。
pub async fn ws_handler(
    State(state): State<AppState>,
    Query(q): Query<WsQuery>,
    ws: WebSocketUpgrade,
) -> Result<Response, AppError> {
    let user_id = state
        .ws_ticket_store
        .consume(&q.ticket)
        .await?
        .ok_or_else(|| AppError::unauthorized("WebSocket ticket 无效或已过期"))?;

    let hub = Arc::clone(&state.ws_hub);
    let notice_store = Arc::clone(&state.notice_store);
    Ok(ws.on_upgrade(move |socket| session(socket, hub, notice_store, user_id)))
}

/// 单条 WS 会话
async fn session(
    socket: WebSocket,
    hub: Arc<WsHub>,
    notice_store: Arc<dyn NoticeStore>,
    user_id: String,
) {
    tracing::info!("WS 已连接: user={}", user_id);
    let (mut sender, mut receiver) = socket.split();

    let (tx, mut rx) = hub.register(&user_id);

    // 连接建立：推送 hello + 权威未读数（冷启动/重连后校准红点）
    let unread = notice_store.unread_count(&user_id).await.unwrap_or(0);
    let hello = json!({"type": "hello", "data": {"userId": user_id}});
    let unread_sync = json!({"type": "unread", "data": {"count": unread}});
    let _ = sender.send(Message::Text(hello.to_string().into())).await;
    let _ = sender
        .send(Message::Text(unread_sync.to_string().into()))
        .await;

    loop {
        tokio::select! {
            // Hub → 客户端（实时通知/未读数）
            outbound = rx.recv() => {
                match outbound {
                    Some(msg) => {
                        if sender.send(msg).await.is_err() {
                            break;
                        }
                    }
                    None => break, // Hub 侧被清理（如单用户超限踢出最旧连接）
                }
            }
            // 客户端 → 服务端（心跳 / 关闭）
            inbound = tokio::time::timeout(IDLE_TIMEOUT, receiver.next()) => {
                match inbound {
                    Ok(Some(Ok(msg))) => match msg {
                        Message::Text(text) => {
                            if parse_type(&text).as_deref() == Some("ping") {
                                let pong = echo_pong(&text);
                                if sender.send(Message::Text(pong.into())).await.is_err() {
                                    break;
                                }
                            }
                        }
                        Message::Ping(payload) => {
                            // 协议层心跳兜底
                            if sender.send(Message::Pong(payload)).await.is_err() {
                                break;
                            }
                        }
                        Message::Close(_) => break,
                        _ => {}
                    },
                    Ok(Some(Err(_))) | Ok(None) => break, // socket 关闭/错误
                    Err(_) => {
                        // 空闲超时：90s 无客户端消息，视为死连接
                        tracing::info!("WS 空闲超时断开: user={}", user_id);
                        let _ = sender.send(Message::Close(None)).await;
                        break;
                    }
                }
            }
        }
    }

    hub.unregister(&user_id, &tx);
    tracing::info!("WS 已断开: user={}", user_id);
}

/// 解析消息 type 字段（解析失败返回 None）
fn parse_type(text: &str) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(text)
        .ok()?
        .get("type")?
        .as_str()
        .map(|s| s.to_string())
}

/// 构造 pong 响应：回显客户端 ts
fn echo_pong(ping_text: &str) -> String {
    let ts = serde_json::from_str::<serde_json::Value>(ping_text)
        .ok()
        .and_then(|v| v.get("ts").cloned())
        .unwrap_or(serde_json::Value::Null);
    json!({"type": "pong", "ts": ts}).to_string()
}

#[cfg(test)]
mod tests {
    use super::{echo_pong, parse_type};

    #[test]
    fn parses_ping_type() {
        assert_eq!(parse_type(r#"{"type":"ping"}"#).as_deref(), Some("ping"));
        assert_eq!(parse_type("not-json"), None);
    }

    #[test]
    fn pong_preserves_timestamp() {
        assert_eq!(
            echo_pong(r#"{"type":"ping","ts":42}"#),
            r#"{"ts":42,"type":"pong"}"#
        );
    }
}
