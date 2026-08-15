//! qgc 角色模块数据模型（DTO）。
//!
//! 供 OpenAPI 文档（utoipa）与前端类型生成使用（`GET /api/qgc/*` 的请求/响应），
//! 字段与 handlers 层实际返回的 JSON 保持一一对应。
//! 纯 WebSocket 事件载荷（`MissionProgress` / `CommandAckPayload`）定义在一级
//! `crate::qgc::mod.rs`（不进 OpenAPI）。

use serde::{Deserialize, Serialize};

/// 飞控遥测快照（`GET /api/qgc/telemetry` 响应 + WebSocket `telemetry` 事件）
///
/// 由 MAVLink 常用消息聚合而来，10Hz 更新（由 `[Gcs] TelemetryHz` 配置）。
/// 角度单位统一为度，距离单位为米，速度为 m/s。
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct QgcTelemetry {
    /// 飞控连接状态（收到心跳后为 true，超时 3 秒复位为 false）
    pub connected: bool,
    /// 飞控系统 ID（MAVLink sysid，来自心跳）
    pub sysid: u8,
    /// 解锁状态（心跳 base_mode 的 SAFETY_ARMED 位）
    pub armed: bool,
    /// 飞行模式名（ArduPilot Copter 模式，如 GUIDED / AUTO / RTL）
    pub mode: String,
    /// 飞行器类型（MAV_TYPE 枚举值，如 2=四旋翼）
    pub vehicle_type: u8,
    /// 滚转角（度）
    pub roll: f32,
    /// 俯仰角（度）
    pub pitch: f32,
    /// 航向（度，0-360）
    pub heading: f32,
    /// 滚转速率（度/秒，ATTITUDE rollspeed）
    pub roll_rate: f32,
    /// 俯仰速率（度/秒，ATTITUDE pitchspeed）
    pub pitch_rate: f32,
    /// 偏航速率（度/秒，ATTITUDE yawspeed）
    pub yaw_rate: f32,
    /// 纬度（度）
    pub lat: f64,
    /// 经度（度）
    pub lon: f64,
    /// 海拔（米，MSL）
    pub altitude: f32,
    /// 相对高度（米，相对起飞点）
    pub relative_alt: f32,
    /// 地速（m/s）
    pub groundspeed: f32,
    /// 空速（m/s）
    pub airspeed: f32,
    /// 爬升率（m/s，正为上升）
    pub climb: f32,
    /// 油门百分比（0~100，VFR_HUD）
    pub throttle: f32,
    /// 飞控 CPU 负载（%，SYS_STATUS load × 0.1）
    pub cpu_load: f32,
    /// 电池电压（V）
    pub voltage: f32,
    /// 电池电流（A）
    pub current: f32,
    /// 电池剩余电量（%，-1 为未知）
    pub battery_remaining: i8,
    /// 电池已消耗电量（mAh，BATTERY_STATUS current_consumed）
    pub battery_consumed_mah: f32,
    /// GPS 定位类型（0=无定位, 2=2D, 3=3D, 4=3D 差分）
    pub gps_fix_type: u8,
    /// GPS 水平定位精度（米，0 为未知）
    pub gps_eph: f32,
    /// 可见卫星数
    pub satellites_visible: u8,
    /// 遥测帧速率（帧/秒，最近 2 秒平均）
    pub packet_rate: f32,
    /// 距上次心跳的毫秒数（连接超时判定用）
    pub last_heartbeat_ms: u64,
}

impl Default for QgcTelemetry {
    /// 默认快照：未连接状态（服务启动时重置使用）
    fn default() -> Self {
        Self {
            connected: false,
            sysid: 1,
            armed: false,
            mode: "UNKNOWN".to_string(),
            vehicle_type: 0,
            roll: 0.0,
            pitch: 0.0,
            heading: 0.0,
            roll_rate: 0.0,
            pitch_rate: 0.0,
            yaw_rate: 0.0,
            lat: 0.0,
            lon: 0.0,
            altitude: 0.0,
            relative_alt: 0.0,
            groundspeed: 0.0,
            airspeed: 0.0,
            climb: 0.0,
            throttle: 0.0,
            cpu_load: 0.0,
            voltage: 0.0,
            current: 0.0,
            battery_remaining: -1,
            battery_consumed_mah: 0.0,
            gps_fix_type: 0,
            gps_eph: 0.0,
            satellites_visible: 0,
            packet_rate: 0.0,
            last_heartbeat_ms: 0,
        }
    }
}

/// 飞控命令请求体（`POST /api/qgc/command`）
///
/// # 支持的命令
/// - `arm`：解锁（MAV_CMD_COMPONENT_ARM_DISARM, param1=1）
/// - `disarm`：锁定（MAV_CMD_COMPONENT_ARM_DISARM, param1=0）
/// - `takeoff`：起飞（MAV_CMD_NAV_TAKEOFF, param7=altitude）
/// - `land`：降落（MAV_CMD_NAV_LAND）
/// - `rtl`：返航（MAV_CMD_NAV_RETURN_TO_LAUNCH）
///
/// 命令经 COMMAND_LONG 发送，飞控以 COMMAND_ACK 回执（WebSocket `command_ack` 事件）。
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct QgcCommandRequest {
    /// 命令名：arm / disarm / takeoff / land / rtl
    pub command: String,
    /// 起飞高度（米，仅 takeoff 使用，缺省 10）
    pub altitude: Option<f32>,
}

/// 飞行模式切换请求体（`POST /api/qgc/mode`）
///
/// 支持 ArduPilot Copter 模式名：stabilize / acro / alt_hold / auto /
/// guided / loiter / rtl / circle / land / drift / sport / poshold / brake / throw；
/// 也接受数字自定义模式 ID（如 "16"）。
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct QgcModeRequest {
    /// 模式名或数字模式 ID
    pub mode: String,
}

/// 航点（任务条目）
///
/// 对应 MAVLink MISSION_ITEM_INT（MAV_CMD_NAV_WAYPOINT），
/// 经纬度为 WGS84 十进制度，高度为相对高度（米）。
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct QgcMissionItem {
    /// 序号（上传时由服务端自动重排，从 1 开始）
    pub seq: u16,
    /// 命令（16 = NAV_WAYPOINT）
    pub command: u16,
    /// 纬度（度）
    pub lat: f64,
    /// 经度（度）
    pub lon: f64,
    /// 相对高度（米）
    pub altitude: f32,
}

/// 任务快照（`GET /api/qgc/mission` 响应）
///
/// 任务协议（MISSION_COUNT / MISSION_ITEM_INT / MISSION_ACK）为异步交互，
/// 上传/下载/清除期间状态为 `uploading` / `downloading` / `clearing`，
/// 完成后回到 `idle`；进度经 WebSocket `mission_progress` 事件实时推送。
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct QgcMission {
    /// 任务状态：idle / uploading / downloading / clearing
    pub state: String,
    /// 航点列表（下载完成后为飞控当前任务）
    pub items: Vec<QgcMissionItem>,
    /// 飞控当前执行的航点序号（-1 表示不在任务中）
    pub current_seq: i16,
}

/// 任务上传请求体（`PUT /api/qgc/mission`）
///
/// 上传 1~100 个航点；序号不要求连续，服务端按数组顺序自动重排。
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct QgcMissionUploadRequest {
    /// 航点列表（1~100 个）
    pub items: Vec<QgcMissionItem>,
}
