//! App 端 WebSocket 实时通知模块
//!
//! 端点：`GET /api/v1/app/ws?token=<JWT>`（升级协议为 WebSocket）
//!
//! 鉴权：WS 握手无法携带自定义 Header，token 通过 query 传递，
//! 升级前校验 JWT，失败返回 401（不建立连接）。
//!
//! ## 通信协议（JSON 文本帧）
//!
//! Client → Server：
//! ```json
//! {"type":"ping","ts":1710000000}
//! ```
//! 客户端心跳，每 30s 一次；服务端 90s 内未收到任何消息则断开连接。
//!
//! Server → Client：
//! ```json
//! {"type":"hello","data":{"userId":"u_xxx"}}
//! {"type":"pong","ts":1710000000}
//! {"type":"notice","data":{"id":"ln_xxx","streamer_id":"st_xxx","streamer_nickname":"陈泽","avatar":null,"title":"主播开播","body":"正在直播：xxx","live_started_at":1710000000,"created_at":1710000000,"read":false}}
//! {"type":"unread","data":{"count":3}}
//! ```
//! - `hello`：连接建立确认（鉴权通过的 user_id）
//! - `pong`：心跳响应，回显 ts
//! - `notice`：实时开播通知（payload 与 HTTP 列表接口的 NoticeItem 字段一致）
//! - `unread`：未读数权威同步（连接建立时推送，客户端用于校准本地计数）

mod handler;

pub use handler::ws_handler;
