# 最近三次提交修改记录（2026-08-13）

本文档详细记录 `fj200c_main`（发动机测控）模块从**三路串口**迁移到**五路串口**的完整修改过程，覆盖最近依次产生的三个提交：

| 提交 | 时间 | 主题 | 主要改动侧 |
|---|---|---|---|
| `e51623f` | 15:17 | 去掉 rust-toolchain.toml，fj200c 串口增加为 5 路 | 后端数据结构层（types / decode / abstract_com / com / handlers）+ 生成类型 |
| `5c43d92` | 15:48 | feat: migrate from three to five serial ports | 前端（composables / store / 组件）+ 配置 & 文档补齐 + 后端 bug 修正 |
| `f4f950a` | 15:57 | 环境参数卡片 8 项，数据源调整为 6 参数映射 | 前端 envParams 布局与数据源映射调整 |

三次提交为同一功能的三个阶段：**先改后端数据结构 → 再同步前端与配置 → 最后校正参数映射与文案**。

---

## 一、提交 e51623f：后端 5 路串口迁移（数据层）

**提交信息**：`去掉rust-toolchain.toml，fj200c串口增加为5路`

### 1.1 删除 rust-toolchain.toml

删除文件内容（原锁定工具链 1.96.0）：

```toml
[toolchain]
channel = "1.96.0"
profile = "default"
```

去掉后不再强制固定工具链版本，跟随系统默认工具链编译。

### 1.2 串口协议规格扩展（`src/fj200c_main/fj200c_main/abstract_com.rs`）

在 `ComSpec` 上新增三种协议规格构造函数：

| 构造函数 | 帧头 | 帧数据区长度 | 帧尾长度 | 对应设备 |
|---|---|---|---|---|
| `adam4015_protocol` | `>`（0x3E） | 57 | 0 | Adam4015 环境采集 |
| `adam4117_protocol` | `>`（0x3E） | 57 | 0 | Adam4117 环境采集 |
| `flux_protocol` | `FF FF` | 14 | 2 | Flux 燃油流量计 |

同时将原 `adam_protocol` 改名为 `adam4015_protocol`。附带对 `data_bits/stop_bits/parity` 三行配置读取做了 rustfmt 格式化（无逻辑变化）。

### 1.3 五端口共享数据与端口启停（`src/fj200c_main/fj200c_main/com.rs`）

核心改动（约 +234 行）：

- **端口 Section 常量**：`ECU_SECTION=COM0`、原 `ADAM_SECTION` 拆分并后移为 `ADAM4015_SECTION=COM1`、`ADAM4117_SECTION=COM2`、`DYNO_SECTION=COM3`、`FLUX_SECTION=COM4`。
- **SharedPortData**：字段由 `ecu/adam/dyno` 三组扩展为 `ecu/adam4015/adam4117/dyno/flux` 五组，每组含 `LatestFrame<256>` 原始帧 + `ArcSwap<Fields>` 解码值。
- **define_com_port! 宏实例**：由 3 个扩为 5 个 —— `ECUCom / Adam4015Com / Adam4117Com / DynoCom / FluxCom`，事件类型对应 `ChannelData::Ecu/Adam4015/Adam4117/Dyno/Flux`。
- **init_all_from_config**：按 `count > 0/1/2/3/4` 门控创建五路端口：
  - `init_adam4015`（新增）：除启动接收线程外，额外 spawn 一个每秒发送 `#010\r` 的轮询线程（ADAM 模块需主动查询才回数据）；
  - `init_adam4117`：由原 `init_adam` 改名而来；
  - `init_flux`（新增），`init_dyno` 门控由 `count > 2` 改为 `count > 3`。
- **CSV 写入线程**：`csv_row_values` 调用签名扩展为 5 参数（ecu、adam4015、adam4117、dyno、flux）。
- **mock 发送器 (start_mock_senders)**：模拟帧分发由 `0/1/2` 三路扩展为 `0~4` 五路，新增 Adam4117（connection_index=2）与 Flux（connection_index=4）两个分支。

### 1.4 解码器拆分（`src/fj200c_main/fj200c_main/decode.rs`）

- 原 `validate_adam/decode_adam` 重命名为 `validate_adam4015/decode_adam4015`；
- 新增 `validate_adam4117/decode_adam4117`：同样解析 `>…\r` 的 ASCII 帧，按 `+` 分隔 8 通道，**通道值除以 1000**（4015 与 4117 量程/缩放不同）；
- 新增 `validate_flux/decode_flux`：帧长 18、帧头 `FF FF`，取偏移 2 处的 u16 小端值作为流量 `ll`。

### 1.5 数据结构（`src/fj200c_main/fj200c_main/types.rs`）

- `AdamFields` 拆分为 `Adam4015Fields` 与 `Adam4117Fields`（均为 `channels: [f64; 8]`）；
- 新增 `FluxFields { ll: f64 }`；
- `ChannelData` 由 3 变体扩为 5 变体：`Ecu / Adam4015 / Adam4117 / Dyno / Flux`；
- `Adam4015Fields` 增加 `csv_entries()`（含中文列名）与 `to_row_values()`；
- `all_csv_entries()` / `csv_row_values()` 均扩为 5 数据源。

### 1.6 handler 与 WS 快照（`src/fj200c_main/handlers.rs`）

- `build_initial_snapshot` 由 3 个 `PortData` 快照扩为 5 个（connection_index 0~4）；
- `set_theme_handler` / `ws_session` 仅做了 rustfmt 格式化。

### 1.7 类型同步（api_docs.rs + openapi.json + generated）

- `src/api_docs.rs`：schemas 注册由 `AdamFields` 替换为 `Adam4015Fields + Adam4117Fields`，新增 `FluxFields`；
- 运行 `cargo test export_openapi` 重新生成 `openapi/openapi.json`（约 +57/- 修改）；
- 运行 `orval` 重新生成 `packages/shared/src/api/generated/model/`：
  - `adamFields.ts` → 重命名为 `adam4015Fields.ts`（类型名同步改）；
  - 新增 `adam4117Fields.ts`、`fluxFields.ts`；
  - `channelData.ts` 的联合类型扩为 5 变体；
  - `index.ts` 更新导出。

---

## 二、提交 5c43d92：前端 + 配置层同步五路串口

**提交信息**：`feat: migrate from three to five serial ports for fj200c_main module`（8 条 bullet 说明）

该提交是对上一提交的收尾：补齐前端、配置文件、部署脚本、帮助文档，并修正 e51623f 引入的门控/索引 bug。新增计划文档 `docs/plan-fj200c-5ports.md` 在本提交中标记「已完成」。

### 2.1 配置与部署

- **`config-fj200c_main.ini`**：`[COM] Count = 3 → 5`；新增 `[COM3]`（COM107，115200）、`[COM4]`（COM109，115200）；`[COM2]` 波特率修正为 9600（Adam4117）。
- **`deploy.bat`**：默认 ini 模板同步 `Count = 5` 并追加 `[COM3]`、`[COM4]` 两节。
- **`AGENTS.md`**：`config-fj200c_main.ini` 描述由「三路串口 ECU/ADAM/DYNO」改为「五路串口 ECU/Adam4015/Adam4117/Dyno/Flux」。

### 2.2 前端（frontend/fj200c_main/）

- **`composables/useBackendPorts.ts`**：
  - 类型导入改 `Adam4015Fields, Adam4117Fields, FluxFields`（替换被删除的 `AdamFields`，修复编译失败）；
  - WS 事件分发 switch 扩为 5 case：`1 → Adam4015`、`2 → Adam4117`、`3 → Dyno`、`4 → Flux`；
  - 新增 `handleAdam4015 / handleAdam4117 / handleFlux` 三个处理函数，分别写入 `envParams[0..4]`、`envParams[4..8]`、`envParams[8]` 与各自 footer 统计字节数。
- **`store/dashboard.ts`**：
  - `FooterStats`：`adamRxBytes` 拆分为 `adam4015RxBytes + adam4117RxBytes`，新增 `fluxRxBytes`；
  - `envParams` 由 7 项扩为 **12 项**（两组 ADAM 通道标注 `(4015)/(4117)` + 燃油流量 + 扭矩转速/扭矩/扭矩功率）；
  - 新增 `fluxData` 状态；`dashboardState.fuelFlow` 数据源由 `envParams[4]` 改为 `fluxData.ll`。
- **`components/StatusBar.vue`**：底部状态栏统计项由 `ADAM` 拆为 `Adam4015/Adam4117`，新增 `Flux` 接收字节数，共 5 路。
- **`components/DashboardStats.vue`**：仪表盘燃油流量表计单位 `r/min → L/h`；顶部注释由「7 项环境参数」改为「12 项环境/测功/流量参数」。
- **`views/GenerateReport.vue`**：报表表头 `大气温度/大气压力` 标注为 `大气温度(4015)/大气压力(4015)`，与 CSV 列名保持一致。

### 2.3 后端修正与完善

- **`com.rs`**（修正 e51623f 的索引 bug）：
  - `init_all_from_config` 门控修正为 `count > 2 / > 3 / > 4`，且 conn_idx 与端口序号一致：adam4117=2、dyno=3、flux=4（此前 adam4117/flux 与前者共用 idx 2，产生重复）；
  - `mock_configs` 由 3 项扩为 5 项（`MOCK_COM0~MOCK_COM4`）；
  - mock 模式下 Dyno 事件 `connection_index: 2 → 3`、Flux 事件 `2 → 4`。
- **`mock.rs`**：`MockProfile` 新增 `Adam4117`、`Flux` 变体；`from_section` 映射 5 个 `MOCK_COM` 段（`MOCK_COM1→Adam`、`MOCK_COM2→Adam4117`、`MOCK_COM3→Dyno`、`MOCK_COM4→Flux`，`MOCK_COM0` 优先于 `ECU` 判断）；新增 `generate_flux_frame()`（18 字节，`FF FF` 头 + u16 小端流量 + 校验和）。
- **`report.rs`**：`build_col_idx` 中大气温度/压力/湿度列名改为 `大气温度(4015)/大气压力(4015)/大气湿度(4015)`，避免 `find()` 匹配到第二组（Adam4117）列。
- **`types.rs`**：
  - `Adam4015Fields::csv_entries` 全部列名加 `(4015)` 后缀，`Adam4117Fields` 加 `(4117)` 后缀（区分两组 8 通道）；
  - `FluxFields::csv_entries` 由 4 项（jkwd/njzs/nj/njgl，与 Dyno 重复）修正为 `[("ll", "燃油流量")]`；
  - CSV 容量 `Vec::with_capacity(64 → 80)`。
- **注释与文档**：`handlers.rs`（三端口→五端口描述）、`main.rs:48`、`routes.rs:79/140`、`embedded_assets.rs:35`、`api_docs.rs:72` 的角色/模块注释统一更新；`help_doc.md` 全文更新：五端口接线图（COM103/COM105 均 9600，COM107/COM109 115200）、模拟运行说明改为 `MOCK_COM0~4` 五路进程内直通发送器（无需虚拟串口与物理连线切换）、`Count=5` 配置示例、FAQ 帧头说明（`>` 对应 Adam4015/Adam4117，`FF FF` 对应 Dyno/Flux）。
- **`openapi.json`**：模拟开关接口 `description` 由「驱动三路解码与推送」改为「驱动五路」。
- **`packages/shared/src/roles.ts`**：`MENU_CONFIG` 中 fj200c_main 角色注释由「ECU/ADAM/DYNO 三路串口」改为「ECU/Adam4015/Adam4117/Dyno/Flux 五路串口」。

### 2.4 新增计划文档 `docs/plan-fj200c-5ports.md`

首次提交 48 行计划文档，含：

- 5 路串口定义表（COM0~COM4、设备、帧格式）；
- 后端变更清单（com.rs / mock.rs / report.rs / config / deploy.bat，标注优先级）；
- 前端变更清单（useBackendPorts.ts / dashboard.ts / StatusBar.vue / DashboardStats.vue）；
- 无需改动项（openapi.json 与 generated 已在 e51623f 完成；fj200c_information 非三路模块不受影响）；
- 验证步骤（`cargo build` + `cargo test export_openapi` + `npm run gen:api` + 前端 build + `Count=5` 后 `cargo run` 检查 WS 5 路事件）。

---

## 三、提交 f4f950a：环境参数卡片 8 项与数据源映射调整

**提交信息**（正文即数据源对照表）：

```
参数        数据源
大气温度    Adam4117 ch0
大气湿度    Adam4117 ch1
大气压力    Adam4117 ch2
进口温度    Adam4015 ch3
燃油流量    Flux
扭矩转速/扭矩/扭矩功率  Dyno
```

结论：前两个提交把环境参数做成 12 项（两组 ADAM 各 4 项重复），本提交确认**只保留一组 8 项卡片**，并指定每组数据唯一来源。

### 3.1 `store/dashboard.ts`

`envParams` 由 12 项收敛为 **8 项**，去掉重复的 `(4015)/(4117)` 括号标注：

```ts
{label: '大气温度', unit: '℃'},      // ← Adam4117 ch0
{label: '大气湿度', unit: '%'},       // ← Adam4117 ch1
{label: '大气压力', unit: 'KPa'},     // ← Adam4117 ch2
{label: '进口温度', unit: '℃'},      // ← Adam4015 ch3
{label: '燃油流量', unit: 'L/h'},     // ← Flux
{label: '扭矩转速', unit: 'r/min'},   // ← Dyno
{label: '扭矩', unit: 'N·m'},         // ← Dyno
{label: '扭矩功率', unit: 'kW'},      // ← Dyno
```

### 3.2 `composables/useBackendPorts.ts`

按新数据源映射重写各 handler 的写入下标：

| handler | 改动前 | 改动后 | 说明 |
|---|---|---|---|
| `adam4015ParamIndices` | `[0,1,2,3]` | 删除 | Adam4015 不再占据前 4 项 |
| `handleAdam4015` | 循环写 `envParams[0..4]` | 只写 `envParams[3] = f.channels[3]` | 仅进口温度取 4015 ch3 |
| `adam4117ParamIndices` | `[0,1,2,3]` | `[0,1,2]` | 只取前 3 通道 |
| `handleAdam4117` | 写 `envParams[4+i]` | 写 `envParams[i]` | 大气温/湿/压占据前 3 项 |
| `handleDyno` | 写 `envParams[9/10/11]` | 写 `envParams[5/6/7]` | 扭矩转速/扭矩/扭矩功率 |
| `handleFlux` | 写 `envParams[8]` | 写 `envParams[4]` | 燃油流量 |

`footerStats` 各端口接收字节数统计保持不变。

### 3.3 `components/DashboardStats.vue`

顶部注释由「12 项环境/测功/流量参数」改为「8 项」并补充数据来源说明（大气温/湿/压取 Adam4117 通道，进口温度取 Adam4015 通道）；布局/表格结构本身未变。

---

## 四、提交间关联总结

1. **e51623f（15:17）**：完成五路串口的后端数据链路（协议规格 → 端口启停 → 解码 → 数据结构 → WS 快照）与 OpenAPI 类型同步，并顺手删除 rust-toolchain.toml；
2. **5c43d92（15:48）**：前端接入五路事件、配置/部署/帮助文档同步，同时修复上一提交中 `init_all_from_config` 门控、mock 事件 `connection_index`、行 `idx` 重复及报表/CSV 列名冲突等 bug，按计划文档收尾；
3. **f4f950a（15:57）**：业务确认数据源映射后，将前端环境参数卡片由 12 项收敛为 8 项，明确大气温/湿/压 = Adam4117（ch0~2）、进口温度 = Adam4015（ch3）、燃油流量 = Flux、扭矩三项 = Dyno。

至此，`fj200c_main` 完成从「ECU/ADAM/DYNO 三路串口」到「ECU/Adam4015/Adam4117/Dyno/Flux 五路串口」的完整迁移，前后端类型、配置、部署脚本、帮助文档全部一致。