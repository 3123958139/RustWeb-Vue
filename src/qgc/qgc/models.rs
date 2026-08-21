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
    /// 返航点纬度（度，HOME_POSITION）
    pub home_lat: f64,
    /// 返航点经度（度，HOME_POSITION）
    pub home_lon: f64,
    /// 返航点海拔（米，MSL）
    pub home_alt: f32,
    /// 距返航点水平距离（米）
    pub distance_home: f32,
    /// 返航点方位角（度，0-360）
    pub bearing_home: f32,
    /// 数传链路本地信号强度（dBm，127 为未知）
    pub radio_rssi: i8,
    /// 数传链路远端信号强度（dBm，127 为未知）
    pub radio_rssi_remote: i8,
    /// 飞行时长（秒，解锁起累计）
    pub flight_time_s: f32,
    /// EKF 状态标志位（位掩码，来自 EKF_STATUS_REPORT.flags；各 bit 表示对应估计器是否健康）
    #[serde(default)]
    pub ekf_flags: u32,
    /// EKF 速度方差（越小越健康）
    #[serde(default)]
    pub ekf_vel_variance: f32,
    /// EKF 水平位置方差
    #[serde(default)]
    pub ekf_pos_horiz_variance: f32,
    /// EKF 垂直位置方差
    #[serde(default)]
    pub ekf_pos_vert_variance: f32,
    /// EKF 罗盘方差
    #[serde(default)]
    pub ekf_compass_variance: f32,
    /// 振动 X 轴（m/s/s RMS）
    #[serde(default)]
    pub vibration_x: f32,
    /// 振动 Y 轴（m/s/s RMS）
    #[serde(default)]
    pub vibration_y: f32,
    /// 振动 Z 轴（m/s/s RMS）
    #[serde(default)]
    pub vibration_z: f32,
    /// RC 接收机通道原始值（PWM 微秒，chan1 起，长度最多 18；未连接或无遥控为 0）
    #[serde(default)]
    pub rc_channels: Vec<u16>,
    /// RC 接收机信号强度（0-100，0 表示遥控器丢失）
    #[serde(default)]
    pub rc_rssi: u8,
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
            home_lat: 0.0,
            home_lon: 0.0,
            home_alt: 0.0,
            distance_home: 0.0,
            bearing_home: 0.0,
            radio_rssi: 127,
            radio_rssi_remote: 127,
            flight_time_s: 0.0,
            ekf_flags: 0,
            ekf_vel_variance: 0.0,
            ekf_pos_horiz_variance: 0.0,
            ekf_pos_vert_variance: 0.0,
            ekf_compass_variance: 0.0,
            vibration_x: 0.0,
            vibration_y: 0.0,
            vibration_z: 0.0,
            rc_channels: Vec::new(),
            rc_rssi: 0,
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
/// - `start`：开始执行任务（MAV_CMD_MISSION_START）
/// - `pause`：暂停任务（MAV_CMD_DO_PAUSE_CONTINUE, param1=0）
/// - `resume`：继续任务（MAV_CMD_DO_PAUSE_CONTINUE, param1=1）
/// - `click_to_go`：随点随行（SET_POSITION_TARGET_GLOBAL_INT，`params`=[lat, lon, alt]）
/// - `move`：键盘/摇杆速度控制（SET_POSITION_TARGET_LOCAL_NED，`params`=[vx, vy, vz] 机体速度 m/s）
///
/// 命令经 COMMAND_LONG 发送，飞控以 COMMAND_ACK 回执（WebSocket `command_ack` 事件）。
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct QgcCommandRequest {
    /// 命令名：arm / disarm / takeoff / land / rtl / start / pause / resume / click_to_go / move
    pub command: String,
    /// 起飞高度（米，仅 takeoff 使用，缺省 10）
    pub altitude: Option<f32>,
    /// 附加参数（click_to_go: [lat, lon, alt]；move: [vx, vy, vz]）
    pub params: Option<Vec<f32>>,
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
/// 对应 MAVLink MISSION_ITEM_INT（MAV_CMD_NAV_WAYPOINT 或 DO 动作命令），
/// 经纬度为 WGS84 十进制度，高度为相对高度（米）。动作条目（action 非 none）
/// 由服务端在航点后自动追加 DO 命令条目（MAV_CMD_DO_SET_SERVO / DO_SET_CAM_TRIGG_INTERVAL）。
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct QgcMissionItem {
    /// 序号（上传时由服务端自动重排，从 1 开始；0 = 首页）
    pub seq: u16,
    /// 命令（16 = NAV_WAYPOINT，183 = DO_SET_SERVO，20001 = DO_SET_CAM_TRIGG_INTERVAL）
    pub command: u16,
    /// 纬度（度）
    pub lat: f64,
    /// 经度（度）
    pub lon: f64,
    /// 相对高度（米）
    pub altitude: f32,
    /// 停留时间（秒，NAV_WAYPOINT param1；航点悬停时长）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hold_time: Option<f32>,
    /// 转弯模式：fixed 定点 / coordinated 协调 / adaptive 自适应（NAV_WAYPOINT param2 转弯半径）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub turn_mode: Option<String>,
    /// 航点动作：none / camera 拍照 / servo 舵机（自动追加 DO 命令条目）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
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

/// 瓦片缓存统计（`GET /api/qgc/tiles/stats` 响应）
///
/// 供前端「离线地图」面板展示缓存占用情况（磁盘 `tiles/` 目录）。
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct TileStats {
    /// 已缓存瓦片数量
    pub count: usize,
    /// 缓存占用磁盘字节数
    pub bytes: u64,
}

/// 数据流频率设置请求体（`POST /api/qgc/stream`）
///
/// 设置遥测广播频率（Hz），模拟器下一拍生效，限幅 1~50Hz。
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct QgcStreamRequest {
    /// 遥测频率（Hz，1~50）
    pub hz: u16,
}

/// 数据流频率响应（`GET/POST /api/qgc/stream`）
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct QgcStreamResponse {
    /// 当前遥测频率（Hz）
    pub hz: u16,
}

/// 遥测 CSV 文件元信息（`GET /api/qgc/telemetry/csv` 列表元素）
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct QgcCsvFile {
    /// 文件名（如 qgc_telemetry_20260821.csv）
    pub name: String,
    /// 文件大小（字节）
    pub size: u64,
    /// 修改时间（Unix 秒）
    pub modified: i64,
}

/// 单个参数项（`GET /api/qgc/param` 列表元素）
///
/// 对应 MAVLink `PARAM_VALUE`，name 为参数名（如 `RTL_ALT`），value 为单精度浮点值。
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct QgcParam {
    /// 参数名（最多 16 字节 ASCII，ArduPilot 约定全大写）
    pub id: String,
    /// 参数值（单精度浮点）
    pub value: f32,
}

/// 参数表列表响应（`GET /api/qgc/param`）
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct QgcParamList {
    /// 参数列表（模拟器维护的 ArduCopter 精简子集）
    pub params: Vec<QgcParam>,
}
