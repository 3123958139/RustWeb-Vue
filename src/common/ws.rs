//! # 公共 WebSocket 事件桥
//!
//! 各角色的 WebSocket 会话主循环同构：订阅全局广播通道，
//! 将事件转发为 JSON 文本帧给客户端，客户端消息仅处理关闭帧。
//! 统一收拢到此公共函数，按角色传入广播发送端。
//!
//! # 性能设计（预序列化广播）
//!
//! 广播通道中传输的是**已序列化的 `Arc<str>`**（JSON 文本），而非事件对象：
//! - 生产端（采集线程）只序列化一次，广播给 N 个订阅者只克隆 `Arc` 指针
//! - `ws_bridge` 只转发，不再对每个客户端重复 `serde_json::to_string`
//! - 事件对象在生产者处构建后立即序列化即被丢弃，零深克隆

use axum::extract::ws::{Message, WebSocket};
use futures_util::{SinkExt, StreamExt};
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{info, trace};

/// 广播通道中传输的事件载荷（预序列化的 JSON 文本）
pub type EventPayload = Arc<str>;

/// 生成角色的全局事件广播通道单例（`static XXX_TX` + `xxx_tx()` getter）
///
/// 各角色事件类型不同，但通道形态一致（容量 1024，满则丢弃最旧事件），
/// 载荷为预序列化的 `Arc<str>`（见 `serialize`），广播只克隆指针。
/// 重复展开时需使用不同标识符。
#[macro_export]
macro_rules! event_broadcast {
    ($tx_ident:ident, $getter:ident) => {
        /// 全局广播通道：服务线程（`std::thread`）写入，WebSocket 任务（`tokio`）读取。
        /// 通道容量 1024，满时丢弃最旧事件（`Lagged` 错误）。
        static $tx_ident: std::sync::OnceLock<
            tokio::sync::broadcast::Sender<crate::common::ws::EventPayload>,
        > = std::sync::OnceLock::new();

        /// 获取全局事件广播发送端（惰性初始化，容量 1024）
        ///
        /// 首次调用创建通道，后续直接克隆 `Sender`（仅增加引用计数）。
        pub fn $getter() -> tokio::sync::broadcast::Sender<crate::common::ws::EventPayload> {
            $tx_ident
                .get_or_init(|| tokio::sync::broadcast::channel(1024).0)
                .clone()
        }
    };
}

/// 序列化事件为广播载荷（生产端调用，只序列化一次）
pub fn serialize<E: Serialize>(event: &E) -> Result<EventPayload, serde_json::Error> {
    serde_json::to_string(event).map(Into::into)
}

/// WebSocket 升级前的 JWT 鉴权
///
/// 浏览器 WebSocket API 不支持自定义头，token 经 `?token=` 查询参数传递；
/// 缺失或无效一律返回 `401 Unauthorized`。
pub fn verify_query_token(
    params: &std::collections::HashMap<String, String>,
) -> Result<(), axum::http::StatusCode> {
    let token = params
        .get("token")
        .ok_or(axum::http::StatusCode::UNAUTHORIZED)?;
    crate::common::jwt::verify_token(token)
        .map(|_| ())
        .map_err(|_| axum::http::StatusCode::UNAUTHORIZED)
}

/// WebSocket 会话主循环：将广播事件转发为 JSON 文本帧
///
/// 使用 `tokio::select!` 同时监听：
/// 1. 客户端发送的消息（仅处理 Close，其余忽略）
/// 2. 广播通道的事件（转发为 JSON 文本帧）
///
/// 事件滞后时（`Lagged`）丢弃旧事件而非阻塞，避免背压。
///
/// # 参数
/// - `tx`: 全局事件广播发送端（`subscribe()` 获取接收端）
/// - `socket`: 已升级的 WebSocket 连接
/// - `log_prefix`: 日志前缀（如 `"[ftj1c]"`），便于区分角色
pub async fn ws_bridge(tx: broadcast::Sender<EventPayload>, socket: WebSocket, log_prefix: &str) {
    ws_bridge_with_initial(tx, socket, log_prefix, None).await
}

/// WebSocket 会话主循环：将广播事件转发为 JSON 文本帧
///
/// 使用 `tokio::select!` 同时监听：
/// 1. 客户端发送的消息（仅处理 Close，其余忽略）
/// 2. 广播通道的事件（转发为 JSON 文本帧）
///
/// 事件滞后时（`Lagged`）丢弃旧事件而非阻塞，避免背压。
///
/// # 参数
/// - `tx`: 全局事件广播发送端（`subscribe()` 获取接收端）
/// - `socket`: 已升级的 WebSocket 连接
/// - `log_prefix`: 日志前缀（如 `"[ftj1c]"`），便于区分角色
/// - `initial_text`: 连接建立时立即发送的 JSON 文本（如当前数据快照）
pub async fn ws_bridge_with_initial(
    tx: broadcast::Sender<EventPayload>,
    socket: WebSocket,
    log_prefix: &str,
    initial_text: Option<String>,
) {
    let mut rx = tx.subscribe();
    let (mut sender, mut receiver) = socket.split();

    if let Some(text) = initial_text {
        if sender.send(Message::Text(text)).await.is_err() {
            return; // 客户端已断开
        }
    }

    loop {
        tokio::select! {
            msg = receiver.next() => {
                match msg {
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(_)) => continue,  // 忽略客户端文本/二进制消息
                    Some(Err(_)) => break,
                }
            }
            event = rx.recv() => {
                match event {
                    Ok(payload) => {
                        // 载荷为预序列化的 Arc<str>，此处仅克隆指针并转成 String 帧
                        if sender.send(Message::Text(payload.to_string())).await.is_err() {
                            break;  // 客户端断开
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        // 客户端接收过慢，丢弃滞后事件而非阻塞
                        // 高频数据源（如串口帧）下 Lagged 属正常现象，降为 trace 避免日志刷屏
                        trace!("{} WebSocket 客户端接收过慢，丢弃 {} 个滞后事件", log_prefix, n);
                        continue;
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
        }
    }

    info!("{} WebSocket 连接已关闭", log_prefix);
}
