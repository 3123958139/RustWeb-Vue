//! # 任务协议状态机（上传 / 下载 / 清除）
//!
//! 任务交互为异步协议（MAVLink mission microservice）：
//!
//! ```text
//! 上传：GCS MISSION_COUNT → 飞控 MISSION_REQUEST_INT(0..n) → GCS MISSION_ITEM_INT(0..n) → 飞控 MISSION_ACK
//! 下载：GCS MISSION_REQUEST_LIST → 飞控 MISSION_COUNT(n) → GCS MISSION_REQUEST_INT(0..n) → 飞控 MISSION_ITEM_INT(0..n)
//! 清除：GCS MISSION_CLEAR_ALL → 飞控 MISSION_ACK
//! ```
//!
//! 状态存于全局 `state::mission()`（`Mutex<MissionState>`），由三处协作：
//! - handler：写入操作请求（经下行通道交给发送线程执行）
//! - 接收线程：解析飞控的请求/应答消息，更新状态
//! - 发送线程：轮询状态推进（发送条目 / 超时复位）
//!
//! 超时约定：进入 `uploading` / `downloading` / `clearing` 后 3 秒无推进
//! 即复位为 `idle` 并记录 `result = timeout`。

use crate::qgc::models::{QgcMission, QgcMissionItem};
use std::time::Instant;

/// 任务状态机
#[derive(Debug, Clone)]
pub struct MissionState {
    /// 任务状态：idle / uploading / downloading / clearing
    pub state: String,
    /// 航点列表（上传中的待发列表 / 下载完成的飞控任务）
    pub items: Vec<QgcMissionItem>,
    /// 飞控侧任务总条目数（上传目标数 / 下载总数）
    pub total: u16,
    /// 已处理条目数（下载时已接收数）
    pub received: u16,
    /// 结果描述：ok / timeout / 错误消息
    pub result: String,
    /// 飞控当前执行航点（模拟器更新，-1 = 不在任务中）
    pub current_seq: i16,
    /// 下一步待发送的条目序号
    pub seq_next: u16,
    /// 最近一次飞控请求的条目序号（上传应答 / 下载推进依据）
    pub last_request_seq: Option<u16>,
    /// 状态超时时刻（None = 不在进行中）
    pub deadline: Option<Instant>,
}

impl MissionState {
    pub fn new() -> Self {
        Self {
            state: "idle".to_string(),
            items: Vec::new(),
            total: 0,
            received: 0,
            result: String::new(),
            current_seq: -1,
            seq_next: 0,
            last_request_seq: None,
            deadline: None,
        }
    }

    /// API 快照（`GET /api/qgc/mission` 响应）
    pub fn snapshot(&self) -> QgcMission {
        QgcMission {
            state: self.state.clone(),
            items: self.items.clone(),
            current_seq: self.current_seq,
        }
    }

    /// 进入一个进行中状态并设置超时
    pub fn begin(&mut self, state: &str, total: u16) {
        self.state = state.to_string();
        self.total = total;
        self.received = 0;
        self.seq_next = 0;
        self.last_request_seq = None;
        self.result = String::new();
        self.deadline = Some(Instant::now() + std::time::Duration::from_secs(3));
    }

    /// 完成（成功 / 失败 / 超时），回到 idle
    pub fn finish(&mut self, result: &str) {
        self.state = "idle".to_string();
        self.result = result.to_string();
        self.deadline = None;
        self.last_request_seq = None;
    }

    /// 超时检查（发送线程每 100ms 调用）
    pub fn check_timeout(&mut self) -> bool {
        if let Some(d) = self.deadline {
            if Instant::now() >= d {
                self.finish("timeout");
                return true;
            }
        }
        false
    }
}
