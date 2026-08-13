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
