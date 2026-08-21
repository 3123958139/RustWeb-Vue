# qgc 模块审查与优化计划

> 审查日期：2026-08-20
> 范围：后端 `src/qgc/` + 前端 `frontend/qgc/`（飞控地面站，MAVLink v2 + UDP + 模拟飞控 + Cesium 3D + 离线瓦片）

## 一、审查结论（Bug 清单）

### 1.1 严重 Bug（真实飞控下功能失效 / 数据错误）

**S1. SET_POSITION_TARGET type_mask 掩码常量全部错位**（`src/qgc/qgc/mavlink.rs`）

| 常量 | 当前值 | 应为（MAVLink POSITION_TARGET_TYPEMASK） |
|---|---|---|
| `POSITION_IGNORE` | 0x07 | 0x07 ✓ |
| `VELOCITY_IGNORE` | `0x07 << 0` = 0x07（与 POSITION 重复） | 0x38（bit3-5） |
| `ACCEL_IGNORE` | 0x38（实为速度位） | 0x1C0（bit6-8） |
| `YAW_IGNORE` | 0x80（实为 AY 加速度位） | 0x400（bit10） |
| `YAW_RATE_IGNORE` | 0x100（实为 AZ 加速度位） | 0x800（bit11） |

- `encode_set_position_global`（随点随行）：mask = 0x1BF 含位置忽略位 → 真实飞控忽略 lat/lon/alt，`click_to_go` 失效
- `encode_set_position_local`（键盘摇杆）：yaw/yaw_rate 位未设置，速度指令不完整
- 模拟器不检查 mask，mock 模式掩盖问题

**S2. 键盘遥控松键后飞机持续飞行**（`Screen.vue` kbdLoop + `simulator.rs` kbd_vel）

- 前端只在有按键时发 `move`，松键后不发 `[0,0,0]` 停止指令
- 模拟器 `kbd_vel` 一旦设置永不清零；真实飞控保持最后速度指令

**S3. 飞行时长累计慢 10 倍**（`process.rs` HEARTBEAT 分支）

- `flight_time_s += 0.1` 写在心跳（1Hz）分支，实际每秒只 +0.1s
- 注释称"100ms 步进"与实现不符

### 1.2 性能问题

**P1. 瓦片 IO 全部阻塞 tokio 线程**（`tiles.rs`）

- `fs::read`/`create_dir_all`/`write`、`stats()` 全目录遍历均为同步调用，直接跑在 async worker 上
- Cesium 批量加载 + 离线保存 8 并发时卡顿
- 修复：`tokio::fs` / `spawn_blocking`；stats 增量计数

**P2. 瓦片无 in-flight 去重**（`tiles.rs`）

- 多客户端并发请求同一未缓存瓦片会重复下载
- 修复：`HashMap<(z,x,y), 进行中任务>` 合并

**P3. `estimateTiles()` 模板中调用两次全量计算**（`OfflineMapPanel.vue`）

- 每次渲染计算两遍瓦片集合（上限 3 万条目）
- 修复：缓存为 computed

### 1.3 中等问题

| # | 位置 | 问题 |
|---|---|---|
| M1 | `process.rs` `_telemetry_hz` | `[Gcs] TelemetryHz` 配置被忽略，广播硬编码 10Hz |
| M2 | `tiles.rs` 写盘 | 瓦片源返回 200 的 HTML 错误页被永久缓存（应校验 PNG magic/Content-Type）；`fs::write` 非原子写 |
| M3 | `handlers.rs` 多处 `let _ = tx.send` | 通道关闭（服务停止竞态）时仍返回 success(true) |
| M4 | `Config.vue` 文案 | 声称"[Mock] 修改立即生效"与实现不符（reload_config 仅校验语法，全部需重启） |
| M5 | `simulator.rs` 放电 | 10Hz × 2%/拍 = 20%/秒，20 秒耗尽，演示观感夸张 |
| M6 | `simulator.rs` SET_MODE | 模式切换不清除 kbd_vel，键盘速度残留 |

### 1.4 小问题 / 优化点

| # | 位置 | 问题 |
|---|---|---|
| L1 | `mavlink.rs` FrameExtractor | 垃圾字节时 `buf.remove(0)` 每字节 O(n) |
| L2 | `config.rs` | `Mock == "true"` 大小写敏感 |
| L3 | `process.rs` | `rssi != 127` 判断未排除 -1（0xFF） |
| L4 | `process.rs` 上传 | 未定位（lat=0）时首页落在 (0,0)，应先校验 connected + GPS |
| L5 | handlers | 上传/下载/清除无状态重入检查 |
| L6 | 安全 | 瓦片 URL 带 `?token=` 进访问日志（与 WS 同机制，知悉） |
| L7 | `useQgcEvents.ts` | 注释"单页面共享一条连接"与实际每组件一条不符 |
| L8 | `Map.vue` | `watch(items, deep)` 编辑 hold_time 等也全量重建航点实体 |

## 二、功能缺口（对照 QGroundControl / Mission Planner）

### 2.1 高优先级（贴合现有半成品 UI / 项目风格）

**G1. 航点属性落地**（最明显的半成品）

- `MissionPanel.vue` 已有 停留时间/转弯模式/动作（拍照/舵机）UI
- 后端 `process.rs` 强制 `NAV_WAYPOINT` 且 `mavlink.rs` "param1..4 = 0" 全部丢弃
- 补：param1=停留时间、param2=转弯半径、param4=偏航；`action=camera/servo` 转 `DO_SET_CAM_TRIGG_INTERVAL` / `DO_SET_SERVO` 条目

**G2. 遥测 CSV 记录**

- 其他 8 模块均有 CSV 录制，qgc 唯一没有
- 加 `[CSV]` 配置 + 接收线程落盘

**G3. 告警系统**

- 大屏显控应有：低电量 / 失联 / 非解锁起飞 / 高度超限（视觉高亮 + Web Speech API 语音播报）

**G4. 命令可靠性：ack 超时重传**

- 命令发完即完，飞控丢包/未应答无感知
- 记录最近命令 + 3s 内无 `COMMAND_ACK` 重发或告警

### 2.2 中优先级（GCS 核心功能缺失）

| # | 功能 | 说明 |
|---|---|---|
| N1 | 参数表读写（PARAM_*） | PID 调参/配置，Mission Planner 最核心功能；先做精简版：`GET/PUT /api/qgc/param` 单参数 + 列表快照 |
| N2 | 传感器健康度 | `EKF_STATUS_REPORT` / `VIBRATION` / `COMPASS_MOT` 解码 + 健康面板 |
| N3 | RC 显示与失控告警 | `RC_CHANNELS` 解码 + 通道条 + 遥控器丢失（rssi=0）告警 |
| N4 | 数据流配置 | 现硬编码 `REQUEST_DATA_STREAM ALL @10Hz`，应支持按流选择/调速 |
| N5 | 2D 视图切换 | Cesium SceneMode 切换，成本极低 |
| N6 | 航程/电量估算 | 总距离/预计时间/电量消耗，前端计算即可 |

### 2.3 低优先级（可后置）

- FPV 视频流（RTSP/UDP H264）——模拟器无视频源
- 校准流程（加速度计/罗盘）——真实飞控才有意义
- Geofence / Rally point 上传
- 多机支持（多 sysid）——learn 机制只认单机
- 手柄 Gamepad API（键盘已有）
- 遥测回放、测量工具、禁飞区图层

## 三、实施计划

### 阶段一：Bug 修复（高优先级）

1. **S1** `mavlink.rs`：修正 5 个 mask 常量值 + 使用处核对
2. **S2** `Screen.vue` 松键发 `[0,0,0]`；`simulator.rs` kbd_vel 加 500ms 无新指令超时清零
3. **S3** `process.rs`：flight_time_s 改按系统时间累计（解锁期间 elapsed）
4. **P1/P2** `tiles.rs`：`tokio::fs` 读写 + 落盘写临时文件 rename + in-flight 去重；stats 加惰性缓存
5. **M1** `process.rs`：广播间隔用 `telemetry_hz` 配置
6. **M2** `tiles.rs`：校验 PNG magic（`\x89PNG`），非图片不缓存
7. **M4** `Config.vue`：文案修正（全部需重启）
8. **M6** `simulator.rs`：SET_MODE 清 kbd_vel

### 阶段二：高优先级功能

9. **G1** 航点属性：`QgcMissionItem` 加 hold_time/turn_mode/yaw 字段 → `encode_mission_item_int` param1/param2/param4；action 转 DO 命令条目（需新增 DO_SET_CAM_TRIGG_INTERVAL / DO_SET_SERVO 编码 + 模拟器处理）
10. **G2** 遥测 CSV：`[CSV] Dir/Enabled` 配置 + 接收线程按行落盘（仿 fj200c 模块）
11. **G3** 告警：后端快照已有数据，前端事件流做阈值检测（低电量/失联/高度）+ 语音播报
12. **G4** ack 重传：`state.rs` 记最近命令 + 发送线程 3s 无 ack 重发一次 + WS 事件

### 阶段三：中优先级功能（后续迭代）

- 参数表读写、传感器健康度、RC 显示、数据流配置、2D 视图、航程估算

## 四、验证方式

1. 后端：`cargo build` + `cargo test`（mavlink 编解码单测、tiles 路由单测）
2. 类型同步：`npm run gen:api`（若有 DTO 变更）→ 前端 vue-tsc 报错处即更新点
3. 前端：`npm run build`（在 `frontend/qgc/`）
4. 联调：`cargo run` + 启动 qgc 服务（Mock=true）→ 仪表盘/地图/航点上传飞行/键盘遥控/命令回执全链路验证
5. 数值断言同步：`src/api_docs.rs` export_openapi 测试注释 + AGENTS.md + README.md（新增端点时）

## 五、注意事项

- 模拟器不检查 type_mask，修复 S1 后需用 mock 模式回归随点随行/键盘控制确认无回归
- 改 DTO 后 `openapi.json` 与 `generated/` 由 `npm run gen:api` 生成，不手改
- 生成代码注释一律由后端 `///` 产生
- 数值类注释（路径/操作断言）改动需三处同步