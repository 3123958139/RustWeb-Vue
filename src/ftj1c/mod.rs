//! # FTJ1C 通信监控模块
//!
//! 从 demo-test3-ftj（Tauri 桌面应用）迁移的 UDP 组播通信监控功能，
//! 适配 RustWeb-Vue 的 Axum + Vue3 架构。
//!
//! ## 架构映射（Tauri → Web）
//!
//! | Tauri 概念 | Web 替代 |
//! |---|---|
//! | `invoke("start_service")` 等命令 | `crate::ftj1c::handlers` 中的 HTTP 端点 |
//! | `emit("udp-data")` | `Ftj1cEvent::UdpData` + `tokio::sync::broadcast` |
//! | `listen("udp-data")` 前端监听 | WebSocket `/api/ftj1c/ws` 推送 |
//! | Tauri 全局 `QuadFrame` | `state` 中全局 `QuadFrame` 实例 |
//! | `mock` feature（编译期） | `[Udp] Mock = true` 配置（运行期开关，默认开启）|
//!
//! ## 模块结构
//!
//! 一级目录仅保留模板骨架，角色专有子模块位于二级目录 `ftj1c/`，
//! 通过下方 `pub use` 再导出，外部路径保持不变（`crate::ftj1c::x`）。
//!
//! | 模块 | 位置 | 用途 |
//! |---|---|---|
//! | `handlers` | 一级 | HTTP 端点 + WebSocket 推送 |
//! | `routes` | 一级 | 子路由定义 |
//! | `service` | 一级 | 服务启动/停止编排 |
//! | `state` | 二级 | 全局状态（运行标志、配置路径、QuadFrame、停止信号） |
//! | `config` | 二级 | config-ftj1c.ini 解析（复用公共 INI 封装） |
//! | `quad_frame` | 二级 | 4 槽无锁共享帧（复用公共泛型实现，95 字节帧） |
//! | `udp` | 二级 | UDP 套接字控制（组播加入、1MB 接收缓冲） |
//! | `process` | 二级 | UDP 工作线程（主备双链 / 单路连接 / 模拟数据源） |
//! | `com` | 二级 | 串口协议构建与坐标转换 |
//!
//! 帧提取器（`FrameExtractor`）、四槽帧缓冲（`QuadFrame`）、工具函数等
//! 通用逻辑位于 `crate::common`，由 fj200c_information / ftj1c 共享。
//!
//! ## 帧协议（与 demo-test3-ftj 一致）
//!
//! ```text
//! Offset  Len  Field        Value
//! ─────────────────────────────────
//! 0-2     3    HEADER       EB 90 5B
//! 3       1    SLOT         01~04
//! 4-7     4    SEQ          LE u32
//! 8-92    85   PAYLOAD      (...)
//! 93-94   2    CHECKSUM     LE u16 (前 93 字节累加和)
//! ─────────────────────────────────
//! Total:  95 bytes
//! ```
//!
//! ## 主备切换机制
//!
//! - 主链（IP11）每帧刷新心跳；备链（IP15）数据仅在主链心跳超时 1 秒后才发射；
//! - 主链恢复后 `try_update` 自动切回主链；4 个槽位独立 CAS 去重。
//!
//! ## 数据流
//!
//! ```text
//! [UDP 主链/备链] → [FrameExtractor] → [QuadFrame] → [broadcast] → [WebSocket] → [Vue 前端]
//!                       ↓
//!                 [串口发送] (3 路)
//! ```

pub mod ftj1c;
pub mod handlers;
pub mod routes;
pub mod service;

// 再导出二级目录子模块，保持既有 `crate::ftj1c::x` 路径不变
pub use ftj1c::{com, config, models, process, quad_frame, state, udp};

use serde::Serialize;
use tokio::sync::broadcast;

/// 全局广播通道：服务线程（std::thread）写入，WebSocket 任务（tokio）读取。
/// 载荷为**预序列化的 `Arc<str>`**（见 `common::ws::serialize`），
/// 生产端序列化一次，广播只克隆 Arc 指针，避免每个客户端重复序列化。
/// 使用 `OnceLock` 保证线程安全的单次初始化。
static FTJ1C_TX: std::sync::OnceLock<broadcast::Sender<crate::common::ws::EventPayload>> =
    std::sync::OnceLock::new();

/// 获取全局事件广播发送端
///
/// # 返回值
/// `broadcast::Sender<EventPayload>` 的克隆实例，可安全跨线程传递。
///
/// # 实现细节
/// - 首次调用时通过 `get_or_init` 创建通道（容量 1024，满则丢弃最旧事件）
/// - 返回 `Sender` 的克隆，允许多个生产者并发发送
/// - `Receiver` 由 WebSocket 任务自行订阅（`subscribe()`）
pub fn ftj1c_tx() -> broadcast::Sender<crate::common::ws::EventPayload> {
    FTJ1C_TX
        .get_or_init(|| {
            let (tx, _rx) = broadcast::channel(1024);
            tx
        })
        .clone()
}

/// 推送给前端的通信监控事件（WebSocket JSON，`type` 字段区分事件类型）
///
/// # 序列化格式
/// 使用 `#[serde(tag = "type")]` 实现内部标签序列化：
/// ```json
/// { "type": "udp_data", "connection_index": 5, ... }
/// ```
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Ftj1cEvent {
    /// UDP 数据事件（对应原 Tauri 的 `udp-data` 事件）
    UdpData(UdpDataPayload),
}

/// UDP 数据事件载荷
///
/// # 字段说明
/// - `connection_index`: 连接序号 0..7，前端映射为第 `connection_index*2` 与 `*2+1` 张卡片
/// - `slot_index`: 槽位序号 0..3，对应 4 个独立数据槽
/// - `source`: 来源标识
///   - `"P"` = 主链（Primary，IP11）
///   - `"S"` = 备链（Secondary，IP15）
///   - `"-"` = 单路（非主备切换模式）
/// - `raw_hex`: 原始数据十六进制（含前缀描述，如 `接收:[95]EB 90 5B ...`）
/// - `ext_hex`: 提取/发送数据十六进制
#[derive(Debug, Clone, Serialize)]
pub struct UdpDataPayload {
    /// 连接序号 0..7，前端映射为第 `connection_index*2` 与 `*2+1` 张卡片
    pub connection_index: usize,
    /// 槽位序号 0..3
    pub slot_index: usize,
    /// 来源："P"=主链、"S"=备链、"-"=单路
    pub source: String,
    pub local_ip: String,
    pub local_port: u16,
    pub remote_ip: String,
    pub remote_port: u16,
    /// 原始数据十六进制（含前缀描述，如 `接收:[95]EB 90 5B ...`）
    pub raw_hex: String,
    /// 提取/发送数据十六进制
    pub ext_hex: String,
}
