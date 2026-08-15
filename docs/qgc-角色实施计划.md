# 新增 qgc 角色（QGroundControl 风格飞控地面站）实施计划

## 决策摘要

- **地图**：Leaflet + OpenStreetMap（新增依赖，需联网加载瓦片）
- **协议**：真实 MAVLink v2（帧格式 + X25 校验 + CRC_EXTRA 表），模拟器也生成真实帧
- **范围**：基础显控 + 地图 + 任务规划（航点编辑/上传/下载/清除）
- **形态**：完全复用现有角色模式 —— 后端 `src/qgc/`（两级目录，仿 ftj1c 线程型角色）+ 前端 `frontend/qgc/`（端口 **5181**，prod 路径 `/qgc`）

## 一、后端

### 1. 权限与角色注册

| 文件 | 改动 |
|---|---|
| `src/common/models.rs` | `Permission` 枚举加 `QgcMonitor` |
| `src/roles.rs` | `ROLE_REGISTRY` 加 `RoleDef{key:"qgc", name:"qgc", permissions:&[QgcMonitor]}`；模块头角色表同步加一行 |
| `src/common/dto.rs` | 复用现有 `ServiceStatus`（无需改） |

### 2. 新建 `src/qgc/`（两级目录，仿 ftj1c 线程型角色）

```
src/qgc/
├── mod.rs            # 骨架 + event_broadcast!(QGC_TX, qgc_tx) + QgcEvent 枚举
├── handlers.rs       # 12 个 utoipa 注解 handler + ws_handler（verify_query_token + ws_bridge_with_initial 推送遥测快照）
├── routes.rs         # qgc_router(db)：/ws 挂中间件外；protected 挂 auth + qgc_permission_middleware(QgcMonitor)
├── service.rs        # start/stop 编排（仿 ftj1c/service.rs：排他启动 + stop_in_background）
├── config-qgc.ini    # 配置样本（[Udp] LocalIp/LocalPort=14550/TargetIp/TargetPort/Mock；[Gcs] SysId=255/CompId=190/HeartbeatMs/TelemetryHz）
├── help_doc.md
└── qgc/              # 二级角色专有目录
    ├── mod.rs
    ├── state.rs      # SERVICE_RUNNING / CONFIG_PATH / STOP_SIGNAL / 遥测快照（供 ws_bridge_with_initial）
    ├── config.rs     # config_singleton!（公共宏）
    ├── models.rs     # QgcTelemetry / QgcCommandRequest / QgcModeRequest / QgcMissionItem / QgcMission（全部 ToSchema）
    ├── mavlink.rs    # MAVLink v2 帧编解码核心
    ├── udp.rs        # UDP 收发（绑定 LocalPort，记录对端地址供命令回发，仿 ftj1c/udp.rs 的 socket2 模式）
    ├── mission.rs    # 任务上传/下载状态机
    ├── simulator.rs  # 模拟飞控（生成真实 MAVLink 帧 + 简单航点飞行模型）
    └── process.rs    # 工作线程：接收解析线程 / 发送线程（心跳+命令队列）/ 模拟线程
```

### 3. MAVLink 核心（`mavlink.rs`）—— 本角色技术重点

- **帧格式**：v2 magic `0xFD` + len/incompat/compat/seq/sysid/compid/msgid(3B) + payload + X25 校验（帧头+载荷+CRC_EXTRA）
- **消息子集**（CRC_EXTRA 实现时以官方 `mavlink/message_definitions/common.xml` 核对）：
  - 解析：`HEARTBEAT(0)` `SYS_STATUS(1)` `ATTITUDE(30)` `GPS_RAW_INT(24)` `GLOBAL_POSITION_INT(33)` `VFR_HUD(74)` `BATTERY_STATUS(147)` `COMMAND_ACK(77)`
  - 发送：`HEARTBEAT` 回复、`REQUEST_DATA_STREAM(66)`/`SET_MESSAGE_INTERVAL`（请求遥测流）、`COMMAND_LONG(76)`（MAV_CMD_COMPONENT_ARM_DISARM=400 / NAV_TAKEOFF=22 / NAV_LAND=21 / NAV_RETURN_TO_LAUNCH=20）、`SET_MODE(176)`（ArduPilot Copter 模式 ID：STABILIZE=0/ALT_HOLD=2/AUTO=3/GUIDED=4/LOITER=5/RTL=6/LAND=9）、任务消息
- **任务协议**（`mission.rs`）：上传 = 发送 `MISSION_COUNT(44)` → 应答 `MISSION_REQUEST_INT(51)` → 逐条 `MISSION_ITEM_INT(73)`（MAV_CMD_NAV_WAYPOINT=16）→ 收 `MISSION_ACK(47)`；下载 = `MISSION_REQUEST_LIST(43)` → `MISSION_COUNT` → 逐条请求；清除 = `MISSION_CLEAR_ALL(45)`；发送线程内带超时（3s）状态机，进度经 WS `mission_progress` 事件广播
- **遥测聚合**：接收线程按 sysid 维护最新快照，`Throttle` 10Hz 广播 `QgcEvent::Telemetry`

### 4. 模拟器（`simulator.rs`）

`[Udp] Mock = true` 时开箱即用：模拟多旋翼（悬停/爬升/航点飞行/降落状态机 + 高斯噪声），编码**真实 MAVLink 帧**经同一解析管道；解锁/起飞/命令/模式切换真实生效；支持按上传航点顺序飞行。

### 5. 生命周期与挂载

- `src/common/service.rs` `stop_all_services_except` 加分支 `if keep_role != Some("qgc") { crate::qgc::service::stop_service(); }`；`src/qgc/service.rs` 启动时调 `stop_all_services_except(Some("qgc"))`
- `src/routes.rs`：`let qgc_routes = crate::qgc::routes::qgc_router(db.clone());` + `.nest("/api/qgc", qgc_routes)`
- `src/main.rs`：`mod qgc;` + dev 静态托管 `.nest_service("/qgc", ServeDir::new("dist-qgc").fallback(...))`
- `src/embedded_assets.rs`：`QgcAssets => "frontend/qgc/dist/"` + `embed_app_routes!("qgc", QgcAssets)`
- `src/database.rs`：种子加第 9 条 `(uuid ...000000000009, "qgc", "qgc@7304.com", "qgc")` + 头部注释表同步

### 6. API 一览（13 新 handler / 10 唯一路径 / 13 新操作）

| 方法/路径 | 说明 |
|---|---|
| `POST /api/qgc/service/start` `stop` / `GET status` | 服务生命周期 |
| `GET/PUT /api/qgc/config` | config-qgc.ini（保存后重启生效） |
| `GET /api/qgc/telemetry` | 遥测快照（含连接/解锁/模式/GPS/电量） |
| `POST /api/qgc/command` | 解锁/锁定/起飞/降落/返航 |
| `POST /api/qgc/mode` | 切换飞行模式 |
| `GET/PUT/DELETE /api/qgc/mission` | 任务下载/上传/清除 |
| `POST /api/qgc/mission/download` | 任务下载（与 GET /mission 共用下载状态机） |
| `GET /api/qgc/help` | help_doc.md |
| `WS /api/qgc/ws?token=` | `telemetry`（10Hz）+ `mission_progress` 事件 |

- `src/api_docs.rs`：paths() 加 13 个 handler（mission 三方法共用 1 路径）、schemas() 加 6 个 DTO、export_openapi 测试加 10 个路径断言
- **数量断言**：`49 → 59` 路径、`61 → 74` 操作（测试注释、AGENTS.md、README.md 同步）

## 二、前端

### 7. 新建 `frontend/qgc/`（端口 5181，ws: true）

复制 ftj1c 骨架（App.vue/main.ts/router/stores/api/Login.vue），二级目录 `src/qgc/`：

```
frontend/qgc/
├── vite.config.ts      # defineAppConfig({app:"qgc", port:5181, ws:true}, __dirname)
├── package.json        # + leaflet ^1.9、@types/leaflet
└── src/qgc/
    ├── qgc.css                 # QGC 深色仪表主题
    ├── api/qgc.ts              # facade（getQgc 封装）+ 手写 QgcEvent WS 类型 + buildWebSocketUrl("/api/qgc/ws")
    ├── composables/useQgcEvents.ts   # WS 连接（fj200c_information 模式：自动重连 + type 分发）
    ├── components/AttitudeIndicator.vue   # 姿态仪（SVG 人工地平线）
    ├── components/HeadingTape.vue / BatteryGauge.vue / AltitudeSpeedGauge.vue
    ├── components/MissionPanel.vue        # 航点列表（增删/排序/上传/下载/清除）
    └── views/
        ├── Monitor.vue     # 仪表盘：姿态/航向/高度/速度/爬升率/电量/GPS/模式/解锁态 + 命令按钮组（解锁·起飞·降落·返航 + 模式下拉）
        ├── Map.vue         # Leaflet 地图：飞机 Marker（按航向旋转）+ 轨迹线 + 点击添加航点 + MissionPanel
        ├── Config.vue      # config-qgc.ini 编辑器
        └── Help.vue
```

路由：`/qgc/monitor` `/qgc/map` `/qgc/config` `/qgc/help`；auth store `allowedRoles: ["qgc"]`。

### 8. 共享包与周边

- `packages/shared/src/roles.ts`：`MENU_CONFIG.qgc`（仪表盘/地图与任务/打开配置/帮助 4 菜单）+ `ROLE_APP_URLS.qgc = {dev:"http://localhost:5181", prod:"/qgc"}`
- `packages/shared/src/template/AppNavbar.vue` 品牌名 switch 加 `case 'qgc': return 'QGC 飞控地面站';`
- 根 `package.json` workspaces 加 `frontend/qgc`
- `build-frontends.ps1`：`$apps` 加 `'qgc'`（9 个改三波 3+3+3）；`deploy.bat`：打印 `http://localhost:3000/qgc`、复制 `config-qgc.ini`

## 三、实施顺序与验证

1. **后端骨架**：Permission/注册表/`src/qgc/` 全部模块（先 mavlink + udp + simulator + service）→ `cargo build` 通过
2. **`npm run gen:api`** → 生成 `generated/api/qgc/qgc.ts` + model；断言 `62 路径 / 77 操作`
3. **前端**：`frontend/qgc/` 全量（api facade → useQgcEvents → 组件 → 4 视图）→ `npm run build` 通过
4. **联调**：dev 模式 `cargo run` + 启动 qgc 服务（Mock=true）→ 仪表盘/地图/航点上传飞行/命令按钮全链路验证
5. **构建**：`cargo build --release --features embedded` 验证内嵌

## 注意事项

- 瓦片支持**离线保存与加载**：后端代理（`GET /api/qgc/tiles/:z/:x/:y`，token 经 `?token=`）从瓦片源（`config-qgc.ini` `[Tiles] Url`，默认 OSM）下载并落盘 `tiles/` 磁盘缓存；地图浏览自动缓存（离线加载），「离线地图」面板可区域批量保存（中心 + 半径 + 缩放级别）与清除。`[Tiles] Url` 支持自定义瓦片源（内网部署可换内网源）
- CRC_EXTRA 值必须按官方 mavlink XML 核对，防校验错导致丢帧
- 任务上传状态机放发送线程内，UDP 无确认时 3s 超时重置为 idle
- 数值类注释（49/61 → 62/77）三处同步改（api_docs.rs 测试注释、AGENTS.md、README.md）