//! # 发动机监控模块（fj200c_information）
//!
//! 从 fj200c_information.informatization（Tauri 桌面应用）迁移的发动机参数通信与监控功能，
//! 适配 RustWeb-Vue 的 Axum + Vue3 架构。
//!
//! 本模块实现了发动机监控的完整数据链路：
//! - **数据采集**：通过串口（`com`）或进程内模拟（`mock`）获取原始字节流
//! - **帧提取**：`frame_extractor` 从字节流中定位帧头、校验帧完整性
//! - **协议解码**：`decode` 将 100 字节帧解码为 28 个工程字段
//! - **数据存储**：`frame_bundle` 缓存最新帧，`csv_writer` 持久化记录
//! - **实时推送**：通过 `tokio::sync::broadcast` 广播事件，WebSocket 推送到前端
//!
//! ## 架构映射（Tauri → Web）
//!
//! | Tauri 概念 | Web 替代 |
//! |---|---|
//! | `invoke("start_service")` 等命令 | `crate::fj200c_information::handlers` 中的 HTTP 端点 |
//! | `emit("frame"/"payload"/"table_data")` | `Fj200cInformationEvent` + `tokio::sync::broadcast` |
//! | `listen("frame")` 前端监听 | WebSocket `/api/fj200c_information/ws` 推送 |
//! | Tauri managed `SharedData` | `state::SharedData::global()` 全局单例 |
//!
//! ## 模块结构
//!
//! 一级目录仅保留模板骨架，角色专有子模块位于二级目录 `fj200c_information/`，
//! 通过下方 `pub use` 再导出，外部路径保持不变（`crate::fj200c_information::x`）。
//!
//! | 模块 | 位置 | 用途 |
//! |---|---|---|
//! | `handlers` | 一级 | HTTP 端点 + WebSocket 推送 |
//! | `routes` | 一级 | 子路由定义 |
//! | `service` | 一级 | 服务启动/停止编排 |
//! | `state` | 二级 | 全局状态（SharedData、运行标志、配置路径） |
//! | `config` | 二级 | config-fj200c_information.ini 解析（复用公共 INI 封装） |
//! | `decode` | 二级 | 100 字节帧协议校验与 28 字段解码 |
//! | `csv_writer` | 二级 | CSV 写入（500ms 批量刷新） |
//! | `csv_sink` | 二级 | CSV 异步写入线程（磁盘 IO 移出采集线程） |
//! | `frame_bundle` | 二级 | 帧数据复合存储（ArcSwap + RwLock） |
//! | `com` | 二级 | 串口控制（serialport 4，实现 IoControl） |
//! | `mock` | 二级 | 进程内模拟数据控制器（实现 IoControl） |
//! | `mock_feeder` | 二级 | 虚拟串口对模式的数据发生器 |
//! | `session` | 二级 | 每连接 IO 会话线程 |
//!
//! 帧提取器（`FrameExtractor`）、四槽帧缓冲（`QuadFrame`）、工具函数等
//! 通用逻辑位于 `crate::common`，由 fj200c_information / ftj1c 共享。
//!
//! ## 关键语法
//!
//! - **`broadcast::Sender`**：tokio 广播通道发送端，`send()` 是同步方法，
//!   可在 `std::thread` 中直接调用，支持多接收者（多个 WebSocket 客户端）。
//! - **`OnceLock`**：线程安全一次性初始化容器（`std::sync::OnceLock`），
//!   用于全局单例（Config、SharedData），保证只初始化一次。
//! - **`#[serde(tag = "type", rename_all = "snake_case")]`**：serde 标签枚举序列化，
//!   使 `Fj200cInformationEvent::Frame` 在 JSON 中自动包含 `"type": "frame"` 字段。

pub mod fj200c_information;
pub mod handlers;
pub mod routes;
pub mod service;

// 再导出二级目录子模块，保持既有 `crate::fj200c_information::x` 路径不变
pub use fj200c_information::{com, config, csv_sink, csv_writer, decode, frame_bundle, mock, mock_feeder, session, state};

use serde::Serialize;
use tokio::sync::broadcast;

/// 全局广播通道：服务线程（`std::thread`）写入，WebSocket 任务（`tokio`）读取。
///
/// 通道容量 1024，满时丢弃最旧事件（`Lagged` 错误）。
/// 载荷为**预序列化的 `Arc<str>`**（见 `common::ws::serialize`），
/// 生产端序列化一次，广播只克隆 Arc 指针，避免每个客户端重复序列化。
static FJ200C_TX: std::sync::OnceLock<broadcast::Sender<crate::common::ws::EventPayload>> =
    std::sync::OnceLock::new();

/// 获取全局事件广播发送端（惰性初始化）
///
/// 首次调用时创建容量 1024 的广播通道，后续调用直接克隆 Sender。
/// `Sender::clone()` 仅增加引用计数，不会创建新通道。
pub fn fj200c_information_tx() -> broadcast::Sender<crate::common::ws::EventPayload> {
    FJ200C_TX
        .get_or_init(|| {
            let (tx, _rx) = broadcast::channel(1024);
            tx
        })
        .clone()
}

/// 推送给前端的引擎事件（WebSocket JSON，`type` 字段区分事件类型）
///
/// 通过 serde 标签枚举自动序列化为带 `"type"` 字段的 JSON，例如：
/// ```json
/// { "type": "frame", "connection_index": 0, "hex": "EB9064...", "frame_type": "试验数据下载", "fields": [...] }
/// ```
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Fj200cInformationEvent {
    /// 帧事件：包含连接索引、原始十六进制、帧类型和解码后的字段列表
    Frame {
        connection_index: usize,
        hex: String,
        frame_type: String,
        fields: Vec<String>,
    },
    /// 原始数据事件：包含连接索引和原始十六进制字符串（200ms 节流推送）
    Payload {
        connection_index: usize,
        hex: String,
    },
    /// 表格数据事件：包含连接索引和 16 个 SharedData 标识字段
    TableData {
        connection_index: usize,
        rows: Vec<TableRow>,
    },
}

/// 表格行：字段名 + 值，用于 `TableData` 事件中的每一行数据
#[derive(Debug, Clone, Serialize)]
pub struct TableRow {
    pub field: String,
    pub value: String,
}

/// 统一硬件 IO 抽象（串口 / 进程内模拟），重导出自公共层
pub use crate::common::io::IoControl;
