# fj200c_main 角色 ECU 通信协议变更指南

> 适用角色：`fj200c_main`（发动机测控，三路串口 ECU / Adam4015 / Adam4117 / Dyno / Flux）
> 本指南**只聚焦 ECU 一路**通信协议的所有可能变化及按部就班的修改步骤；Adam/Dyno/Flux 仅在涉及共享机制（帧提取、CSV、WebSocket）时作为关联影响提示，不展开其协议细节。
> 文档中的 `file:line` 引用基于当前代码基线，作为定位锚点；协议变更后请以实际代码为准。

---

## 0. 文档目的与前置知识

### 0.1 为什么需要这份指南

ECU 数据从串口字节流到前端仪表盘，跨 **5 个层次**，任何协议改动都必须联动：

```
[串口字节流]
   │ ① 帧校验 + 解码（decode.rs / mock.rs）
   ▼
[EcuFields 结构体]（types.rs，带 #[utoipa::ToSchema]）
   │ ② WebSocket 广播（ChannelData::Ecu）
   ▼
[前端 store]（dashboard.ts，Object.assign 写入）
   │ ③ 视图绑定（ECUStatus.vue / ExperimentView.vue / 仪表盘 / 曲线）
   ▼
[CSV 录制 + 报表]（types.rs csv_entries / report.rs）
   │ ④ 类型同步（npm run gen:api 生成 @shared/api/generated）
   ▼
[前端编译期类型]（ecuFields.ts / faultCodeFlags.ts）
```

ECU 还有一条**反向通道**（前端 → ECU 指令帧），独立于上述接收帧，详见 §2.5。

### 0.2 读者前提

- 了解 Rust 基本语法（struct / enum / 闭包），理解 `utoipa::ToSchema` + `serde(rename_all="camelCase")` 如何驱动 OpenAPI 与 JSON 字段名。
- 了解 Vue 3 + Pinia 基础（store / 组件绑定）。
- 已读项目 `AGENTS.md` 的「前后端类型同步」「常见陷阱」章节。

### 0.3 总原则（务必遵守）

1. **单点真相**：协议字段的"英文字段名 + 中文列名 + 解码公式"的权威定义在后端 `src/fj200c_main/fj200c_main/types.rs`，前端视觉绑定只是引用。
2. **先后端后前端**：永远先改后端 DTO 与解码，再 `npm run gen:api` 重新生成类型，让 `vue-tsc` 编译报错**精准指出**所有需要同步的前端调用点，而不是凭记忆改。
3. **模拟与真实走同一条链路**：`mock.rs` 生成的模拟帧与真实设备帧共用 `decode.rs` 的 `validate_ecu` / `decode_ecu`，因此协议改动后**必须同步改模拟帧**，否则模拟模式无法验证新协议。
4. **禁止手改生成文件**：`packages/shared/src/api/generated/**` 与 `openapi.json` 由工具生成，协议变更后只运行 `npm run gen:api` 重新生成。

---

## 1. ECU 协议现状速查（基线）

### 1.1 两种 ECU 帧格式（关键区分）

ECU 方向有两套完全不同的帧，**不要混淆**：

| 名称 | 方向 | 帧头 | 长度 | 校验位置 | 校验算法 | 代码位置 |
|---|---|---|---|---|---|---|
| **接收帧（遥测）** | ECU → 主机 | `EB 90 2A` | 42 字节 | 末字节（索引 41） | 前 41 字节累加和 mod 256 | `decode.rs:27-48`（`validate_ecu`） |
| **指令帧（控制）** | 主机 → ECU | `EB 90 10` | 16 字节 | 末字节（索引 15） | 前 15 字节累加和 mod 256 | `ControlPanel.vue:38-77`（`buildBaseFrame`/`sendConfig`） |

校验函数都写在 `decode.rs` 顶部常量区：`ECU_HEADER = [0xEB,0x90,0x2A]`、`ECU_FRAME_LEN = 42`（`decode.rs:22-24`）。

### 1.2 接收帧（42 字节）逐字节映射表

这是协议变更的**核心参照表**。`decode_ecu`（`decode.rs:93-186`）按固定偏移小端读取。

| 字节偏移 | 原始类型 | 解码公式 | 输出字段（EcuFields） | 说明 |
|---|---|---|---|---|
| 0–2 | — | 帧头 `EB 90 2A` | — | `validate_ecu` 校验 |
| 3 | u8 | 直接读取 | `_count`（**被丢弃**） | 帧序号，解码中未使用 |
| 4 | u8 | `v / 100.0` | `mach_number` | 飞行马赫数回传 |
| 5 | u8 | 直接 | `working_voltage` | 工作电压（V） |
| 6–7 | u16 LE | 直接 | `altitude` | 海拔高度（m） |
| 8–9 | u16 LE | 直接 | `ng_speed` | 燃气发生器转速 Ng |
| 10–11 | u16 LE | `v - 273.0` | `exhaust_temp` | 排气温度（℃），开尔文→摄氏 |
| 12–13 | u16 LE | `v/10 - 273.0` | `intake_temp` | 进气温度（℃） |
| 14–15 | u16 LE | 直接 | `np_speed` | 动力涡轮转速 Np |
| 16–17 | u16 LE | `v / 100.0` | `throttle` | 油门开度（%） |
| 18 | u8 | `engine_status_str()` | `engine_status` / `engine_status_u8` | 状态码 → 中文 + 原值 hex |
| 19 | u8 | `cmd_exec_str()` | `cmd_exec_status` / `cmd_exec_u8` | 指令执行状态 → 中文 + 原值 hex |
| 20–21 | u16 LE | 按位展开 | `fault_code1` + `fault_codes` 的 fc1 16 位 | 自检/起动故障 |
| 22–23 | u16 LE | 按位展开 | `fault_code2` + `fault_codes` 的 fc2 16 位 | 运行故障 |
| 24–25 | u16 LE | `v/10 - 273.0` | `oil_temp` | 滑油温度（℃） |
| 26–27 | u16 LE | 直接 | `fuel_pressure` | 燃油压力 |
| 28 | u8 | 低 5 位按位 | `accessory_status` + `stop_solenoid`/`fuel_pump`/`oil_pump`/`starter`/`wheel_load_status` | 附件状态位 |
| 29 | u8 | `v / 100.0` | `oil_pressure` | 滑油压力（MPa） |
| 30–31 | u16 LE | `v/10 - 273.0` | `exchanger_outlet_temp` | 换热器出口滑油温度（℃） |
| 32–33 | — | **未使用**（保留/对齐间隙） | — | 解码未读 |
| 34–37 | 4×u8 | hex 拼接 | `fingerprint_code` | 特征码，固定 `AB CD 12 34` |
| 38 | u8 | 直接 | `frame_count` | **显示的帧计数**（注意非索引 3） |
| 39–40 | — | **未使用** | — | 解码未读 |
| 41 | u8 | 累加和校验 | — | `validate_ecu` 校验 |

> 状态/指令码的中文映射在 `decode.rs:51-87`（`engine_status_str` / `cmd_exec_str`）。
> 故障位语义在 `types.rs:80-139`（`FaultCodeFlags` 27 个布尔字段，对应 fc1 16 位 + fc2 16 位中的 27 位）。

### 1.3 指令帧（16 字节，主机 → ECU）

由前端 `ControlPanel.vue` 的 `buildBaseFrame`（`ControlPanel.vue:38-53`）构造，经 `POST /api/fj200c_main/service/command` 下发。后端 `service.rs:134`（`send_command`）只做"长度≥16 + 存字符串"，`com.rs:392-398` 周期线程只覆写索引 3（序号）与索引 15（校验和），**原样透传**——即后端不解析指令语义，命令码语义完全由前端约定。

| 字节偏移 | 含义 | 构造逻辑（ControlPanel.vue） |
|---|---|---|
| 0 | `0xEB` | 帧头 |
| 1 | `0x90` | 帧头 |
| 2 | `0x10` | 帧类型固定值 |
| 3 | 帧序号 | 0–255 循环（`frameSeq`） |
| 4 | 马赫数 ×100 | `machNumber*100` 限幅 0–100 |
| 5 | 轮载状态 | 0/1 |
| 6–7 | 海拔高度（**大端** u16） | `(Altitude>>8)&0xFF, Altitude&0xFF` |
| 8 | 保留 | 0 |
| 9 | **命令码** | `CMD_BYTE` 表（见下） |
| 10–11 | 油门占空比（**大端** u16，仅 cmd=0xD1） | `throttleDuty*100` |
| 12–14 | 保留 | 0 |
| 15 | 校验和 | 前 15 字节累加和 mod 256 |

> ⚠️ **字节序陷阱**：指令帧的高低字节（海拔、油门）用**大端**，而接收帧全部用**小端**。修改协议时两侧要保持一致，否则上下行解析错位。
> 命令码表定义在 `ControlPanel.vue:20-30`（`CMD_BYTE`）：`0x91`空中起动 / `0xA1`起动 / `0xB1`冷运转 / `0xC1`停止冷运转 / `0xD1`恒定油门 / `0xE1`停车 / `0xF1`油路排气 / `0x00`空白 / `0x10`自检。

### 1.4 EcuFields 字段清单（types.rs:14-74）

共 27 个字段（含 `fault_codes` 嵌套结构 27 布尔位）。字段名经 `#[serde(rename_all="camelCase")]` 序列化为驼峰 JSON（如 `ng_speed` → `ngSpeed`），经 WebSocket 发给前端。`///` 注释会进入 `openapi.json` 的 schema description，请按"最终用户文档"对待。

### 1.5 配置项（config-fj200c_main.ini）

| 段落 | 键 | 默认值 | 作用 | 改后 |
|---|---|---|---|---|
| `[COM]` | `Count` | `5` | 串口路数（ECU/Adam4015/Adam4117/Dyno/Flux 占 COM0–4） | 需重启 |
| `[Connection0]`（ECU） | `PORTNAME`/`BaudRate`/`DataBits`/`StopBits`/`Parity` | `COM1`/`115200`/`8`/`1`/`0` | ECU 串口参数 | 需重启 |
| `[MOCK]` | `SimulationMenu` | `true` | 是否启用模拟数据源 | 需重启 |
| `[REPORT]` | `StatePoints` | — | 报表状态点 | 需重启 |
| `[CSV]` | `Dir` | `csv` | CSV 输出目录 | 运行期生效 |

串口参数读取在 `abstract_com.rs:191-207`（缺省 `COM1 / 115200 / 8N1`）。

### 1.6 WebSocket 广播结构

- 事件类型手写于 `frontend/fj200c_main/src/fj200c_main/api/fj200c_main.ts:27-57`：`PortDataEvent`（`type:"port_data"`，含 `connection_index` 0=ECU…4=Flux、`hex` 原始帧、`fields: ChannelData`）。
- `ChannelData` 枚举在 `types.rs:177-189`，ECU 变体为 `Ecu(EcuFields)`。
- 握手：连接后先收 5 个 `PortDataEvent` 快照数组，之后为单个事件（`useBackendPorts.ts:79-86`）。

### 1.7 关键文件地图

**后端**

| 文件 | 职责 | 协议敏感点 |
|---|---|---|
| `fj200c_main/fj200c_main/decode.rs` | 帧校验 + 解码 | `validate_ecu`(L27)、`decode_ecu`(L93)、状态/指令中文映射(L51-87) |
| `fj200c_main/fj200c_main/mock.rs` | 模拟帧生成 | `generate_ecu_frame`(L106-194) |
| `fj200c_main/fj200c_main/types.rs` | 字段结构体 + CSV 列定义 | `EcuFields`(L14)、`FaultCodeFlags`(L79)、`csv_entries`(L222)、`fmt_field`(L292)、`all_csv_entries`(L423) |
| `fj200c_main/fj200c_main/com.rs` | 串口收发 + 指令透传 | 发送线程覆写 seq/checksum(L392-398) |
| `fj200c_main/fj200c_main/state.rs` | 运行期状态 + 默认指令帧 | `ecu_send_data`(L42) 默认 `"EB9010..."` |
| `fj200c_main/fj200c_main/report.rs` | 报表生成 | `build_col_idx`(L102) 若报表用 ECU 字段 |
| `fj200c_main/fj200c_main/config.rs` | 配置解析 | 缺省 COM 节 |
| `fj200c_main/fj200c_main/abstract_com.rs` | 串口参数读取 | 缺省值(L191-207) |
| `fj200c_main/service.rs` | 指令下发 | `send_command`(L134) 长度校验 |
| `src/api_docs.rs` | OpenAPI 聚合 | `gen:api` 自动更新 |

**前端**

| 文件 | 职责 | 协议敏感点 |
|---|---|---|
| `src/fj200c_main/api/fj200c_main.ts` | WS 事件类型 + api facade | `PortDataEvent`(L27)、`sendCommand`(L86) |
| `src/fj200c_main/store/dashboard.ts` | 仪表盘 store | `ecuData` 初始化(L42)、`dashboardState`(L127)、`chartData`(L161) |
| `src/fj200c_main/composables/useBackendPorts.ts` | WS 单例 + 分发 | `handleEcu` Object.assign(L155)、分发(L62-77) |
| `src/fj200c_main/components/ECUStatus.vue` | ECU 实时参数面板 | 22 项字段绑定(L20-100) |
| `src/fj200c_main/views/ExperimentView.vue` | 试验数据大字网格 | 字段绑定(L102-122)、附件(L162-168)、故障(L34-47) |
| `src/fj200c_main/components/FaultDisplay.vue` | 故障码色块 | fc1/fc2 标签数组(L24-58) |
| `src/fj200c_main/components/ControlPanel.vue` | 控制指令面板 | `buildBaseFrame`+校验(L38-77)、`CMD_BYTE`(L20-30) |
| `src/fj200c_main/views/GenerateReport.vue` | 报表 | 表格列绑定（手改） |
| `src/fj200c_main/views/Data.vue` | CSV 浏览 | 列**动态**由表头驱动（自动） |
| `packages/shared/src/api/generated/model/ecuFields.ts` | 生成类型 | **禁止手改** |

---

## 2. 变更分类与按部就班清单

下面每一类都给出：**后端改动 → 前端改动 → gen:api → 验证**。通用操作流程见 §3。

### 2.1 帧格式变化（帧头 / 帧尾 / 长度 / 字节序 / 校验算法）

**场景**：ECU 厂商变更帧头字节、帧总长、累加和算法（如改为 CRC16、XOR）、或高低字节序。

**后端改动**：
1. `decode.rs:22-24`：改 `ECU_HEADER` / `ECU_FRAME_LEN` 常量。
2. `decode.rs:27-48`（`validate_ecu`）：改帧头比对、长度判断、校验计算（如 `frame[..41].iter().sum()` 改为 CRC16）。**注意**：模拟帧 `mock.rs:191-192` 的校验必须与这里**完全一致**——建议把校验提取成共享函数 `fn ecu_checksum(frame:&[u8])->u8`，两处都调用，避免漂移。
3. 若改字节序：修改 `decode.rs:96` 的 `u16_le` 闭包为 `u16_be`（受影响的全部多字节字段：altitude/ng_speed/exhaust_temp/intake_temp/np_speed/throttle/oil_temp/fuel_pressure/exchanger_temp/fc1/fc2）。
4. 若帧总长变化：同步更新 `ECU_FRAME_LEN`，且 `mock.rs` 的 `vec![0u8; 42]`（L108）与 `state.rs` 默认帧长度也要对齐。

**前端改动**：
- 接收帧格式变化**无需改前端**解析（前端只收 JSON 后的 `EcuFields`）。但若改了 WebSocket 的 `PortDataEvent` 结构本身，则改 `api/fj200c_main.ts:27-57` 与 `useBackendPorts.ts:62-77` 分发。

**gen:api**：帧格式改动若不新增/改名 DTO 字段，类型不变 → 可跳过；字段变化走 §2.2。

**验证**：`cargo run` 后端 → 前端模拟模式 → WS 收帧正常、校验通过、仪表盘更新（§4）。

### 2.2 新增 ECU 字段

**场景**：协议新增一个参数（如"涡轮后温度"），需进解码、JSON、CSV、前端展示。

**后端改动（顺序）**：
1. `types.rs:14-74` `EcuFields` 加字段，带 `///` 注释（进 OpenAPI）：
   ```rust
   /// 涡轮后温度（℃）
   pub turbine_out_temp: f64,
   ```
2. `decode.rs:93-186` `decode_ecu`：在帧中找到源字节偏移，按既有 `u16_le` 闭包读取并填入结构体（注意避开 §1.2 的保留字节 32-33/39-40，或重新规划偏移）。
3. `mock.rs:106-194` `generate_ecu_frame`：在对应偏移写入模拟值（正弦+噪声），否则模拟帧该字段恒为 0 且校验仍正确（因为 0 也参与累加和）。
4. `types.rs:222-280` `EcuFields::csv_entries`：**追加**一行 `("turbineOutTemp", "涡轮后温度")`。CSV 列数会 +1（见 §5 漂移点：实际列数非 64）。
5. `types.rs:292-351` `fmt_field`：新增 `match` 分支格式化该字段（`_ => String::new()` 兜底已存在，但建议显式处理）。
6. 若报表用到：`report.rs:102` `build_col_idx` 同步；`types.rs:455-512` 报表 DTO 同步。

**前端改动**：
1. `npm run gen:api` 自动更新 `packages/shared/src/api/generated/model/ecuFields.ts`（`gen:api` 后 `vue-tsc` 会因 store 缺少字段而报类型错 → 引导你到下一步）。
2. `store/dashboard.ts:42-102`：`ecuData` 初始 `reactive` 对象补全新字段（否则 TS 类型缺失）。
3. 展示绑定（按需选）：
   - `ECUStatus.vue:20-100` 加一项 `<el-descriptions-item>` 绑定 `ecuData.turbineOutTemp`；
   - `ExperimentView.vue:102-122` 大字网格同步加；
   - 若进仪表盘/曲线：`DashboardStats.vue` / `ChartPanel.vue:26-32` / `dashboard.ts:127-133` 聚合逻辑。
4. `useBackendPorts.ts:155-161` 的 `Object.assign(state.ecuData, f)` **无需改**（自动带新字段）。

**gen:api**：`npm run gen:api`（= `cargo test export_openapi && orval`）。

**验证**：§4。

### 2.3 删除 / 重命名字段

**删除**：
- 后端：`types.rs` 删字段 → `decode_ecu` 删读取（或保留读取丢弃）→ `csv_entries`/`fmt_field` 删条目 → `mock.rs` 删写入（可保留字节填 0）。
- 前端：`gen:api` 后 `vue-tsc` 报错点即所有引用处（`ECUStatus.vue`/`ExperimentView.vue`/`dashboard.ts`），逐一删除绑定。

**重命名**（如 `ng_speed` → `ng_rpm`）：
- 后端：改 `types.rs` 字段名 + `serde` 字段名（`#[serde(rename="ngRpm")]` 或改 `rename_all` 推断）+ `csv_entries` 的英文键 + `fmt_field` 的 `match` key。
- ⚠️ 改名会破坏**历史 CSV 文件**的列名兼容性，谨慎操作；如需向后兼容，保留旧键做别名映射。
- 前端：`gen:api` → 所有 `ecuData.ngSpeed` 引用（`vue-tsc` 报错引导）改为 `ecuData.ngRpm`。

### 2.4 改字段类型 / 单位 / 解码公式（偏移或缩放变化）

**场景**：原 `u16 LE` 改为 `u32 LE`；原单位℃改为 K；原 `v-273` 改为 `v/10-273`；偏移挪动 1 字节。

- 后端：`decode.rs:93-186` 修改闭包偏移与公式（**逐字段核对 §1.2 表**）。`mock.rs` 对应写入公式同步（`L128-188` 各字段的硬编码范围/缩放）。`types.rs` 字段类型若变（`f64`↔`u32`）则同步，`fmt_field` 小数位数（`{:.0}`/`{:.1}`/`{:.2}`）按需调整。
- 前端：`gen:api` 后若类型变化 → 修正 `dashboard.ts` 与组件绑定；单位变化需在 `ECUStatus.vue`/`ExperimentView.vue` 的中文标签同步（如"排气温度（K）"）。

### 2.5 命令码 / 指令帧格式变化

**场景**：新增控制命令、命令码重排、指令帧字段布局变化（如新增"点火使能"字节）。

**前端改动（主要）**：
1. `ControlPanel.vue:20-30` `CMD_BYTE` 表：增删/改命令码值。
2. `ControlPanel.vue:38-77` `buildBaseFrame` / `sendConfig`：调整帧布局（帧头 `0xEB,0x90,0x10`、保留字节、大端高低字节、校验和循环）。**务必与上位机 ECU 约定一致**。
3. 新增输入控件：`ControlPanel.vue` 模板 `v-model` 到 `store.controlPanel`，并在 `store/dashboard.ts` 加对应状态。

**后端改动**：
- `service.rs:134`（`send_command`）长度校验 `frame.len() < 16`：若帧长变化，改阈值。
- `com.rs:392-398`：发送线程固定覆写索引 3（序号）与 15（校验和）。**若新帧格式校验和不在索引 15，必须改这里**，否则发出去校验错。
- `state.rs:42-48` 默认指令帧 `"EB901000000000000000000000000000"`（16 字节 hex 32 字符）：若帧长变化，改默认串长度，否则 `parse_hex` 长度不符导致发送线程 `continue` 跳过。
- 后端对指令帧**不解析语义**——命令码含义完全由前端定义，ECU 设备端解释。

**gen:api**：指令通道不涉及 DTO 字段 → 通常可跳过（仅 `sendCommand` 入参为 `{hex}` 字符串，不变）。

**验证**：前端点按钮 → `StatusBar.vue` 看"最后发送帧"hex → 后端日志确认下发字节数正确 → 用串口助手或模拟模式回环验证 ECU 响应。

### 2.6 波特率 / 串口参数变化

**场景**：ECU 改波特率（如 57600）、校验位、端口名。

- `config-fj200c_main.ini` 的 `[Connection0]`（ECU 对应 COM0）：改 `PORTNAME`/`BaudRate`/`DataBits`/`StopBits`/`Parity`。
- 若需代码级缺省值变化：`abstract_com.rs:191-207` 改 `get_or` 默认值。
- **需重启服务**（配置非热加载，见 `AGENTS.md` 常见陷阱）。

### 2.7 模拟数据范围 / 特征码变化

**场景**：模拟帧的马赫/转速量程调整，或特征码（指纹码）由固定 `AB CD 12 34` 改为其他值。

- `mock.rs:106-194` `generate_ecu_frame`：
  - 量程：`L128`（马赫 30±60）、`L129`（电压）、`L132`（高度 2000±8000）、`L133`（Ng 5000±5500）、`L134`（排气 5730±6000）、`L135`（进气 2730±500）、`L136`（Np 5000±6000）、`L137`（油门 40±50）、`L170`（滑温）、`L171`（燃压）、`L183`（滑压）、`L184`（换热器）等硬编码范围。
  - 状态/指令周期：`L140-150`（发动机状态循环）、`L154-161`（指令执行状态循环）。
  - 故障概率：`L164`（fc1 5%）、`L166`（fc2 8%）。
  - **特征码**：`L187-188` 固定 `f[34]=0xAB, f[35]=0xCD, f[36]=0x12, f[37]=0x34`，需与真实 ECU 一致。
- 改后无需改其他层，因为模拟帧复用 `decode_ecu`/`validate_ecu`。

### 2.8 CSV 列序 / 报表字段联动

- 任何 ECU 字段增删都会改变 `EcuFields::csv_entries`（`types.rs:222`）顺序，进而改变 `all_csv_entries`（`types.rs:423`）整体列序。
- 前端 `Data.vue` 用 **CSV 表头动态驱动列**（`columns` 由文件首行决定），因此旧文件旧列名、新文件新列名互不冲突，**无需手改**。
- 报表相关（`GenerateReport.vue` 表格列绑定）若引用具体字段，需手改；后端报表 DTO 在 `types.rs:452-512`，走 `gen:api` 更新。
- ⚠️ 列数变化后，**务必更新 §5 的"实际 CSV 列数"说明**与 `all_csv_entries` 上方注释（`types.rs:422` 当前误写"共 64 列"）。

---

## 3. 通用操作流程（顺序不可颠倒）

```
1. 改后端 DTO（types.rs EcuFields / FaultCodeFlags）
        + decode.rs（decode_ecu 偏移/公式）
        + mock.rs（generate_ecu_frame 同步写入）
        + 必要时 csv_entries / fmt_field / report.rs
2. 改后端指令通道（仅 §2.5 场景）：
        ControlPanel.vue 命令码/帧布局 → service.rs 长度校验
        → com.rs 覆写字节 → state.rs 默认帧长度
3. 运行 npm run gen:api
        （= cargo test export_openapi 生成 openapi.json
           + orval 生成 packages/shared/src/api/generated/**）
4. 前端 npm run build（或 vue-tsc）
        → 编译报错点 = 所有需同步的字段引用位置
        → 按报错改 ECUStatus.vue / ExperimentView.vue
          / dashboard.ts / GenerateReport.vue
5. 启动验证（§4）
6. 若新增了 API 路径或操作：更新 src/api_docs.rs 的
        export_openapi 测试断言（当前 68 路径 / 86 操作）
        + AGENTS.md / README.md 中同步数字
```

> 关键：**第 3 步之后 `vue-tsc` 的报错就是你的修改清单**。不要凭记忆去找绑定位置——让编译器告诉你。

---

## 4. 验证清单

执行顺序：后端启动 → 模拟模式 → 前端联调。

**后端**
- [ ] `cargo run` 编译通过；`cargo test export_openapi` 通过（路径/操作数断言未因改动被破坏）。
- [ ] 启动进入模拟模式（`[MOCK] SimulationMenu = true`，需重启生效）。
- [ ] 后端日志无 `ECU frame checksum` 调试告警（`decode.rs:41`）——说明模拟帧校验通过。

**前端**
- [ ] `npm run dev`（或 `npm run build`）通过，`vue-tsc` 无残留类型错。
- [ ] 浏览器打开 `/fj200c_main/monitor`，WS 连接后 DashboardStats / ECUStatus 实时刷新。
- [ ] 点导航栏「模拟运行」→ `StatusBar.vue` 显示"模拟运行中"徽章；断线 1.5s 后自动重连（`useBackendPorts.ts`）。
- [ ] 控制面板发一条指令（`ControlPanel.vue`）：`StatusBar.vue` 的"最后发送帧" hex 与预期帧一致；后端日志确认下发字节数正确。
- [ ] 故障码：模拟概率触发时，`FaultDisplay.vue` 与 `ExperimentView.vue` 的 fc1/fc2 色块正确点亮。
- [ ] CSV：点保存数据 → `csv/` 目录生成文件，列名与 §1.7 / `csv_entries` 一致；`Data.vue` 打开能正确分列。
- [ ] 报表：`GenerateReport.vue` 选 CSV 生成 4 表 + `window.print()` 正常。

---

## 5. 已知注意事项（实现漂移点）

以下为调研中发现的、与注释/`AGENTS.md` 表述**不一致**之处。按本指南改协议时请留意，避免踩坑：

1. **CSV 实际列数 ≠ 64**。
   `types.rs:422` 注释写"共 64 列"，实际 `EcuFields::csv_entries` 55 列 + Adam4015 8 + Adam4117 8 + Dyno 3 + Flux 1 = **75 列**。`state.rs` 多处注释也写"64 列"。改动后请以 `all_csv_entries().len()` 为准更新注释。

2. **Adam 解码未 ÷1000**。
   `decode.rs:247-300` 的 `decode_adam4015`/`decode_adam4117` 注释称"除以 1000 转 V"，但实际 `parse_adam`（`decode.rs:200-245`）直接把 7 位 ASCII 数值当 `f64` 使用，未做 mV→V 转换（除以 1000 的代码被注释掉）。若真实 Adam 帧确为 mV，需补上 `/1000.0`，否则前端环境参数显示偏大 1000 倍。

3. **Dyno / Flux 接收帧的校验和未生效**。
   `decode.rs:306-340` 的 `validate_dyno`/`validate_flux` 只校验帧头 `FF FF` 与长度，**未校验** mock 帧末 2 字节的累加和（mock 帧 `L236`/`L254` 写了校验和但后端不验）。若协议要求严格校验，需在此补 `u16_le(16) == sum` 判断。

4. **接收帧 `f[3]` 被丢弃，显示帧计数取 `f[38]`**。
   `decode.rs:98` 把 `frame[3]` 读入 `_count` 后丢弃，`frame_count` 实际来自 `frame[38]`（`decode.rs:143`）；mock 帧 `L124-125` 同时写了两处且值相同（seq）。若真实 ECU 的帧序号不在 `f[38]`，显示会错位。

5. **`[COM] Count` 已满 5 路（COM0–4）**。
   Adam/Dyno/Flux 等占满；若要为 ECU 新增一路独立串口（如 ECU-A/ECU-B），需扩展 `ComSpec` 枚举（`abstract_com.rs`）与 `connection_index`，并同步前端 `PortDataEvent.connection_index` 语义（当前 0=ECU…4=Flux）。

6. **指令帧字节序与接收帧相反**。
   接收帧全部小端（LE），而指令帧海拔/油门用大端（BE，`ControlPanel.vue:47,50`）。协议变更时两侧务必保持一致，否则上下行解析错位——这是最易踩的坑。

7. **后端对指令帧完全透传**。
   命令码语义只在 `ControlPanel.vue:20-30` 的 `CMD_BYTE` 约定，后端 `com.rs`/`service.rs` 不解析；ECU 设备端解释命令码。改命令含义时只动前端 + 通知设备端，无需改后端逻辑（除 §2.5 提及的长度/校验位置）。

---


