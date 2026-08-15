# QGC 飞控地面站帮助文档

复刻 QGroundControl 核心功能的飞控地面站：通过 **MAVLink v2 协议**（UDP）
与飞控通信，开箱即用（默认模拟飞控）。

## 快速开始

1. 点击「启动服务」——默认 `[Udp] Mock = true`，进程内模拟飞控立即开始
   发送真实 MAVLink v2 遥测帧（心跳 1Hz、遥测 10Hz）
2. 「监控」页查看姿态、位置、速度、电池等实时遥测
3. 「地图」页点击地图添加航点 → 「上传任务」→ 切换到 AUTO 模式，
   模拟飞控依次飞向各航点（地图实时显示飞行轨迹）
4. 「配置」页可修改 UDP / GCS 参数（重启服务后生效）

## 连接真实飞控

1. 将 `config-qgc.ini` 的 `[Udp] Mock` 改为 `false`，重启服务
2. 飞控数传模块（如 SiK 数传）设为 UDP 客户端，目标指向本机
   `127.0.0.1:14550`（地面站监听端口 `LocalPort`）
3. 命令将发送到**学习到的对端地址**（最近一次收包来源，自动适配）；
   未收到任何包时回退到 `TargetIp:TargetPort`

## 功能说明

- **遥测监控**：姿态（滚转/俯仰/航向）、位置（经纬度/海拔/相对高度）、
  速度（地速/空速/爬升率）、电池（电压/电流/电量）、GPS 状态、帧速率
- **飞控命令**：解锁 / 锁定 / 起飞（指定高度）/ 降落 / 返航，
  飞控以 COMMAND_ACK 回执（`command_ack` 事件显示结果码）
- **模式切换**：支持 ArduPilot Copter 全部常用模式
  （STABILIZE / ACRO / ALT_HOLD / AUTO / GUIDED / LOITER / RTL / CIRCLE /
  LAND / DRIFT / SPORT / POSHOLD / BRAKE / THROW）
- **任务规划**：航点上传（自动补首页）、下载（GET 任务快照）、清除；
  上传/清除结果经 `mission_progress` 事件实时推送
- **WebSocket**：`/api/qgc/ws?token=` 推送 `telemetry`（10Hz）/
  `mission_progress` / `command_ack` 三类事件

## 配置文件

`config-qgc.ini`（修改后需重启服务）：

| 节 | 键 | 默认 | 说明 |
|---|---|---|---|
| Udp | Mock | true | 模拟飞控开关 |
| Udp | LocalPort | 14550 | 地面站监听端口 |
| Udp | TargetIp | 127.0.0.1 | 命令回退目标 IP |
| Udp | TargetPort | 14551 | 命令回退目标端口 |
| Gcs | SysId | 255 | 地面站系统 ID |
| Gcs | CompId | 190 | 地面站组件 ID |
| Gcs | HeartbeatMs | 1000 | 地面站心跳周期（毫秒） |
| Gcs | TelemetryHz | 10 | 遥测推送频率（Hz） |

## 协议说明

- 兼容 MAVLink v1 / v2 帧（v2 帧含签名时跳过签名块，不校验签名）
- 已知消息严格校验 CRC（X25，CRC_EXTRA 表来自官方 c_library_v2）
- 任务协议（MISSION_COUNT / MISSION_ITEM_INT / MISSION_REQUEST_INT /
  MISSION_ACK / MISSION_CLEAR_ALL）为异步交互，3 秒无推进判定超时
- 下载任务时首条（seq=0）为首页，上传时服务端自动补首页（当前位置）
