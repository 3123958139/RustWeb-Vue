# fj200c_main 前后端业务逻辑审查报告

日期：2026-08-13
范围：`src/fj200c_main/**`（后端 13 个文件）+ `frontend/fj200c_main/src/**`（前端 15 个组件/视图/composable）
状态：已修复（详见各条目"修复"）

---

## 严重 Bug

### 1. 报表"推力/燃油流量"恒为 0 — report.rs 下标错位

- 文件：`src/fj200c_main/fj200c_main/report.rs`
- 现象：`rows_16` 只填充 `r[1],r[2],r[4],r[9],r[10],r[11]`（121-127 行），但取均值读 `avg(7)`（推力）、`avg(8)`（燃油流量）（230-238 行）。下标 7/8 从未赋值 → 推力、燃油流量、标准耗油率、设计点推力/耗油率全部为 0。
- 根因：从 WPF 固定列 CSV（第 7/8 列为推力/燃油流量）迁移到"按表头查找"时只迁移了部分列（燃气发生器转速Ng/排气温度/燃油压力/大气参数），推力/燃油流量未迁移。`col_ryyl`（燃油压力）写入 `r[4]` 后从未使用，`col_dqsd`（湿度）也未使用。
- 修复：改用表头查找——推力列映射 CSV 中 Dyno 的"扭矩"列，燃油流量映射 Flux 的"燃油流量"列；删除死列 `r[4]`。
- 注意：若试车台实际推力来自独立推力传感器（当前 CSV 无此列），需在录制 CSV 中补充该列并调整 `build_col_idx`。

### 2. 报表接口 panic — rename_file_name 双重 unwrap

- 文件：`src/fj200c_main/fj200c_main/report.rs:99` + `src/common/utils.rs:131-141`
- 现象：`rename_file_name` 内 `GlobalVar::global().unwrap()` 与 `gv.get("PathCSV").unwrap()` 双重 unwrap。GlobalVar 仅在 `start_service` / `save_experiment_info` 初始化，PathCSV 仅在服务启动时写入。未打开过 Monitor 页（未自动启动服务）且未保存过试验信息 → 直接进"报表生成"页即 panic（请求无响应/500）。与 handlers.rs 注释"无需服务启动，选中 CSV 即可生成"矛盾。
- 修复：`rename_file_name` 改为返回 `Result<String, String>`，`PathCSV` 缺省回落 `"csv"`；report.rs 调用处改为 `?`。

### 3. 修改 config-fj200c_main.ini 后重启服务不生效

- 文件：`src/fj200c_main/fj200c_main/config.rs` + `src/fj200c_main/service.rs:30`
- 现象：配置全局量基于 `OnceLock`，`set_global` 第二次调用必然返回 Err 被 `let _ =` 吞掉。服务 stop/start 多少次都使用首次加载的旧配置，与 AGENTS.md"修改后需重启生效"不符（重启也不生效）。
- 修复：`fj200c_main/config.rs` 全局量改为 `RwLock<Option<Config>>`，`set_global` 支持重复覆盖；服务重启时重新加载。

### 4. 停止服务后模拟状态不同步

- 文件：`src/fj200c_main/service.rs:57-80`
- 现象：`stop_service` 停掉 mock 发送线程但不复位 `SIMULATION_MODE`，也不广播 `SimulationState` 事件 → 前端 StatusBar 一直显示"模拟运行中"徽章，实际无数据；其他 WS 客户端（多标签页）状态全过期。
- 修复：`stop_service` 内若存在 mock 发送线程则复位 `SIMULATION_MODE=false` 并广播 `SimulationState{simulating:false}`。

### 5. 服务未运行时录制 CSV 只有表头无数据行

- 文件：`src/fj200c_main/service.rs:90-149` + `src/fj200c_main/fj200c_main/com.rs:230-280`
- 现象：写行循环在 `start_processing_thread`（仅服务启动时创建）内；`toggle_csv_recording` 不检查运行状态，只开模拟运行（未启动服务）时点"保存数据" → 文件只有表头，直到服务启动才补写。
- 修复：`toggle_csv_recording` 改为返回 `Result`，开始录制前校验 `is_running()`，未运行返回错误"服务未运行，无法开始录制"（停止录制不受限）。

---

## 中等问题

### 6. 每帧 ERROR 日志刷屏

- 文件：`src/fj200c_main/fj200c_main/decode.rs:71,24-29`
- 现象：`decode_ecu` 对每一帧合法数据打 `error!`（ECU 100ms/帧 = 10 条/秒）；`validate_ecu` 校验失败也打 `error!`。
- 修复：合法帧 hex 日志降为 `trace!`，校验失败降为 `debug!`。

### 7. 类型漂移：jkwd 未同步（openapi.json / generated 过期）

- 现象：commit 85c84a2 已从后端 `DynoFields` 删除 `jkwd`，但 `openapi/openapi.json` 与 `packages/shared/src/api/generated/model/dynoFields.ts` 仍为 required jkwd；前端 `useBackendPorts.ts:178` `f.jkwd ?? 0` 恒为 0（store.dashboard.ts:101 同步残留）。
- 修复：重新执行 `npm run gen:api` 拉平类型，删除前端 jkwd 残留。mock 帧中 jkwd 字节保留（原始帧数据，仅不解码）。

### 8. 默认配置下服务"启动成功"但五路串口全失败 + ECU 发送线程刷错

- 现象：默认 `[COMx] PORTNAME = COM101/103/...` 为不存在的端口，`AbstractCom::new` 返回 Err 但服务仍返回"启动成功"；ECU 发送线程对 Err 端口每 100ms 报一次 `error!`。
- 修复：`init_ecu` 仅在串口打开成功（base Ok）时才启动发送线程，杜绝日志刷屏（端口失败仍有日志，见 `abstract_com.rs:133`）。

---

## 次要问题（已修复 / 留待后续）

### 9. send_command 不校验 hex（已修复）
- `service::send_command` 增加 `parse_hex` + 长度 ≥ 16 校验，非法输入返回明确错误，不再静默丢弃。

### 10. 仪表盘单位错误（已修复）
- `DashboardStats.vue` 测功机功率单位 `W` → `kW`（mock njgl≈15-40，实际为 kW 量级）。
- 燃油流量 gauge 单位 `L/h` 与 help_doc"0–6000 脉冲/min"不一致 —— 保留，待业务确认（Flux 计量单位）。

### 11. WS 初始快照 hex 为 256 字节补零帧（未修复，留待后续）
- `com.rs` 将帧存入 256 字节定长缓冲，`handlers.rs` 快照 hex 输出 512 字符，连接时 RxBytes 每端口多计 256 字节。仅影响状态栏统计的初始值，不影响实时数据（实时事件使用真实帧长）。

### 12. Adam 帧提取器固定 58 字节（未修复，需真机验证）
- `abstract_com.rs:37-40` frame_data_len=57 + 1 字节 `>` 头。真实 4015/4117 返回长度若含符号/指数会跨帧错位。需真机比对后调整。

### 13. dyno/flux 校验不查 checksum（未修复）
- `decode.rs` validate_dyno/validate_flux 仅查帧头，mock 生成的 16 位校验和无人验证。真机协议确认后补充。

### 14. 同秒内快速开→关→开录制会截断覆盖同名文件（未修复）
- 录制文件名只有秒级精度 + `CsvWriter::create` truncate。建议文件名加毫秒或序号。

---

## 前端确认无问题的部分

- WS 单例/引用计数/重连、初始快照数组分发
- Pinia 状态映射（envParams 通道对应关系与 CSV 表头一致）
- ControlPanel 帧构造与后端校验和算法一致（frame[15] = 0..14 累加和）
- CSV 手写解析器（引号转义/跨行）、报表 15 项 basicInfo 对齐、打印样式
