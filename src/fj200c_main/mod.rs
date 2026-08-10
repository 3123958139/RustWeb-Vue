pub mod fj200c_main;
pub mod handlers;
pub mod routes;
pub mod service;

// 再导出二级目录子模块，保持既有 `crate::fj200c_main::x` 路径不变
pub use fj200c_main::{abstract_com, com, config, decode, mock, report, state, types};

use crate::fj200c_main::types::ChannelData;
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::broadcast;

static FJ200C_MAIN_TX: std::sync::OnceLock<broadcast::Sender<crate::common::ws::EventPayload>> =
    std::sync::OnceLock::new();

pub fn fj200c_main_tx() -> broadcast::Sender<crate::common::ws::EventPayload> {
    FJ200C_MAIN_TX
        .get_or_init(|| broadcast::channel(1024).0)
        .clone()
}

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
