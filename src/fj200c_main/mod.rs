//! # fj200c_main 模块（发动机测控）
//!
//! 发动机测控面板后端：五路串口（ECU / Adam4015 / Adam4117 / Dyno / Flux）的
//! 实时采集与指令下发、帧解码、CSV 64 列录制、试验信息管理、报表生成，
//! 并通过 WebSocket 向前端广播实时数据。
//!
//! # 模块结构（两级目录）
//!
//! 一级目录仅保留模板骨架，业务实现在二级目录 `fj200c_main/` 中：
//!
//! - `mod.rs` — 模块声明 + 再导出 + 全局广播通道 + WebSocket 事件类型
//! - `handlers.rs` — HTTP 处理器（服务启停/指令/配置/CSV/试验/报表/主题）
//! - `routes.rs` — 路由注册（`/api/fj200c_main/*`）
//! - `service.rs` — 服务线程生命周期管理（启动/停止/状态）
//! - `fj200c_main/` — 业务实现：串口通信（`com.rs` / `abstract_com.rs`）、
//!   帧解码（`decode.rs`）、模拟数据源（`mock.rs`）、配置（`config.rs`）、
//!   全局状态（`state.rs`）、字段类型（`types.rs`）、报表（`report.rs`）
//!
//! # 数据流
//!
//! 串口/模拟线程 → 帧解码 → `ChannelData` → 预序列化 `Arc<str>` →
//! 广播通道（`FJ200C_MAIN_TX`）→ WebSocket 任务推送给前端。
//! 指令下发：前端 → `send_command` → 帧构造 → ECU 串口发送。
pub mod fj200c_main;
pub mod handlers;
pub mod routes;
pub mod service;

// 再导出二级目录子模块，保持既有 `crate::fj200c_main::x` 路径不变
pub use fj200c_main::{abstract_com, com, config, decode, mock, report, state, types};

use crate::fj200c_main::types::ChannelData;
use serde::Serialize;
use std::sync::Arc;

// 全局广播通道：服务线程写入，WebSocket 任务（tokio）读取。
// 由公共宏 `event_broadcast!` 生成（容量 1024，载荷为预序列化的 `Arc<str>`）。
crate::event_broadcast!(FJ200C_MAIN_TX, fj200c_main_tx);

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Fj200cMainEvent {
    PortData {
        connection_index: usize,
        hex: String,
        /// 解码字段以 `Arc<ChannelData>` 传递，广播/序列化时只克隆指针，避免每帧深拷贝
        fields: Arc<ChannelData>,
    },
    SimulationState {
        simulating: bool,
    },
    ThemeState {
        #[serde(rename = "isDark")]
        is_dark: bool,
    },
    CsvRecordingState {
        recording: bool,
    },
}
