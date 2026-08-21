//! # qgc 角色模块（飞控地面站）
//!
//! 复刻 QGroundControl 的核心功能：通过真实 MAVLink v2 协议经 UDP 与飞控
//! （ArduPilot / PX4 兼容）通信，提供遥测监控、地图航点规划、命令与模式
//! 控制。`config-qgc.ini` 的 `[Udp] Mock = true`（默认）时启动**模拟飞控**
//! （进程内多旋翼仿真，生成真实 MAVLink v2 帧），开箱即用无需硬件。
//!
//! ## 模块结构
//!
//! 一级目录仅保留模板骨架（`mod.rs` / `handlers.rs` / `routes.rs` /
//! `service.rs` + `config-qgc.ini` + `help_doc.md`），角色专有实现位于
//! 二级目录 `qgc/`，通过下方 `pub use` 再导出，外部路径保持 `crate::qgc::x`：
//!
//! | 模块 | 位置 | 用途 |
//! |---|---|---|
//! | `handlers` | 一级 | HTTP 端点 + WebSocket 推送 |
//! | `routes` | 一级 | 子路由定义 |
//! | `service` | 一级 | 服务启动/停止编排 |
//! | `models` | 二级 | DTO（进 OpenAPI，utoipa `ToSchema`） |
//! | `mavlink` | 二级 | MAVLink v2 帧编解码（消息子集 + CRC 表） |
//! | `config` | 二级 | config-qgc.ini 解析（复用公共 INI 封装） |
//! | `csv` | 二级 | 遥测 CSV 录制（按天分割，`[CSV] Dir` 配置） |
//! | `state` | 二级 | 全局状态（运行标志、停止信号、遥测/任务快照、下行通道） |
//! | `udp` | 二级 | UDP 链路（监听 + 对端学习 + 命令回退目标） |
//! | `mission` | 二级 | 任务协议状态机（上传/下载/清除） |
//! | `simulator` | 二级 | 模拟飞控（多旋翼仿真，真实 MAVLink v2 帧） |
//! | `process` | 二级 | 工作线程（接收 / 发送 / 任务推进 / 10Hz 广播） |
//!
//! ## 数据流
//!
//! ```text
//! [飞控/模拟器 UDP] → [FrameExtractor] → [遥测快照 RwLock] → [broadcast 10Hz] → [WebSocket] → [Vue]
//!      ↑                              → [任务状态机 Mutex] ── mission_progress 事件 ─┘
//!      └── [发送线程] ← outbound mpsc ← [HTTP handler]
//! ```

pub mod handlers;
pub mod qgc;
pub mod routes;
pub mod service;

// 再导出二级目录子模块，保持既有 `crate::qgc::x` 路径不变
pub use qgc::{config, csv, mavlink, mission, models, process, simulator, state, tiles, udp};

use serde::Serialize;

// 全局广播通道：服务线程（std::thread）写入，WebSocket 任务（tokio）读取。
// 由公共宏 `event_broadcast!` 生成（容量 1024，载荷为预序列化的 `Arc<str>`）。
crate::event_broadcast!(QGC_TX, qgc_tx);

/// 推送给前端的飞控事件（WebSocket JSON，`event` 字段区分事件类型）
///
/// # 序列化格式
/// 使用 `#[serde(tag = "event")]` 实现内部标签序列化：
/// ```json
/// { "event": "telemetry", "connected": true, ... }
/// ```
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum QgcEvent {
    /// 遥测快照（10Hz，连接断开时推送一次 disconnected 状态）
    Telemetry(models::QgcTelemetry),
    /// 任务协议进度（上传/下载/清除状态与结果）
    MissionProgress(MissionProgress),
    /// 命令回执（COMMAND_ACK）
    CommandAck(CommandAckPayload),
}

/// 任务协议进度事件载荷（不进 OpenAPI）
#[derive(Debug, Clone, Serialize)]
pub struct MissionProgress {
    /// 任务状态：idle / uploading / downloading / clearing
    pub state: String,
    /// 总条目数（含首页）
    pub total: u16,
    /// 已处理条目数
    pub received: u16,
    /// 结果描述：ok / timeout / 错误消息
    pub result: String,
    /// 飞控当前执行航点序号（-1 = 不在任务中）
    pub current_seq: i16,
}

/// 命令回执事件载荷（不进 OpenAPI）
#[derive(Debug, Clone, Serialize)]
pub struct CommandAckPayload {
    /// 原始命令（MAV_CMD 枚举值）
    pub command: u16,
    /// MAV_RESULT 结果码
    pub result: u8,
    /// 结果码名称（ACCEPTED / DENIED / FAILED 等）
    pub result_name: String,
}
