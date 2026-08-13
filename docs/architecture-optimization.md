# 架构优化执行计划（2026-08-13）

> 依据架构分析报告（排除用户有意保留项：种子密码固定 123456、测试为零、无迁移体系）。
> 状态：执行完成项打勾。

## #2 后端：csv_writer 完全重复

- **现状**：`src/common/csv_writer.rs`（113 行）与 `src/fj200c_information/fj200c_information/csv_writer.rs`（154 行）逻辑逐行一致，仅注释详略不同；fj200c_main 已用公共版。
- **方案**：
  1. `csv_sink.rs:16` 改为 `use crate::common::csv_writer::CsvWriter;`
  2. 删除 `src/fj200c_information/fj200c_information/csv_writer.rs`
  3. 二级 `mod.rs` 去掉 `pub mod csv_writer;`，一级 `mod.rs` 再导出列表去掉 `csv_writer`，模块表格注释同步更新

## #3 后端：config 单例样板 ×2

- **现状**：`src/fj200c_information/fj200c_information/config.rs` 与 `src/ftj1c/ftj1c/config.rs` 逐字重复（`OnceLock<Config>` + `global()` / `set_global()`）。fj200c_main 为 `RwLock<Option<Config>>` 热替换变体，语义不同，保留。
- **方案**：`src/common/config.rs` 新增 `#[macro_export] macro_rules! config_singleton`，两个角色改用宏展开。

## #4 后端：WS broadcast 单例 ×3

- **现状**：三个角色 `mod.rs` 各自手写 `static XXX_TX: OnceLock<Sender<EventPayload>>` + `xxx_tx()` getter（容量 1024）。
- **方案**：`src/common/ws.rs` 新增 `#[macro_export] macro_rules! event_broadcast`，三处替换。

## #5 后端：服务停止骨架 ×2

- **现状**：fj200c_information / ftj1c 的 `stop_service()` 同构：`set_stopping(true)` → 置停止信号 → `spawn(join → RUNNING=false → set_stopping(false) → log)`。fj200c_main 为同步停止变体，保留。
- **方案**：`src/common/service.rs` 新增 `stop_in_background(runtime, running, set_stop, log_msg)`，两个角色调用。

## #6 后端：WS handler token 校验 ×3

- **现状**：三个角色 `ws_handler` 手写 `?token=` 提取 + `jwt::verify_token` + 401 映射。
- **方案**：`src/common/ws.rs` 新增 `verify_query_token(&HashMap<String,String>) -> Result<(), StatusCode>`，三处替换。

## #7 前端：模板骨架收敛（style.css / App.vue / devDeps）

- **style.css**：6 份 SHA256 全同（`B030014C`：admin / fj200c_information / ftj1c / fw100 / fw150 / protocol_generator，326 行）→ 移至 `packages/shared/src/style.css`，各应用 `main.ts` 改 `import "@shared/style.css"`，删本地副本。city3d（暗黑变体）/ fj200c_main（精简版）保留本地。
- **App.vue**：8 份逻辑完全相同（AppNavbar + router-view + initAuth，hash 差异仅注释）→ 抽 `packages/shared/src/template/AppShell.vue`（用 `getAppAuthStore()` 获取当前应用 store，8 个应用均已 `registerAuthStoreGetter`），各应用 App.vue 变薄封装。
- **package.json devDeps**：8 份 `devDependencies` 完全一致（@types/node / @vitejs/plugin-vue / typescript / vite / vue-tsc）→ 提升到根 `package.json`，子包删除该字段（npm workspaces hoist 保证 `node_modules/.bin` 可用，AGENTS.md 规定只在根目录 install）。
- **不动**：`utils/responsive.ts`（已是 re-export）、`stores/auth.ts` / `views/Login.vue` / `types/index.ts` / `main.ts` / `index.html` / `vite.config.ts`（薄封装或参数差异，收益低）。

## #8 前端：fw100/fw150 台账面板业务重复

- **现状**：`Panel.vue` 138 行仅类名前缀 / API facade / 列宽不同；`api/fw150.ts` 与 `api/fw100.ts` 同构。
- **方案**：抽 `packages/shared/src/template/LedgerPanel.vue`（props：`roleKey` / `permissionKey` / `api: { getItems() }`，类型用结构性鸭子类型 `LedgerRow { id, name, category, status }`），两个应用 Panel.vue 变薄封装，删除各自 scoped 样式。

## #11 后端：embedded_assets 24 条路由手写

- **现状**：8 应用 × 3 条路由（`/x`、`/x/`、`/x/*path`）逐条手写，rust-embed 结构体逐一手写。
- **方案**：两个 `macro_rules!`（`embed_assets!` 生成结构体、`embed_app_routes!` 生成路由），消灭漏配风险。

## #12 后端：session.rs 高频日志

- **现状**：`session.rs:308` 每 200ms `info!("{:?}", event.clone())`，高频路径刷日志。
- **方案**：降为 `trace!`，去掉多余 `clone()`。

## #13 文档修正

- `packages/shared/src/index.ts` 头部注释「被 5 个前端引用」已过时（实际 8 个），列表补全。
- fj200c_main 为唯一无 `types/` / `utils/` 目录、单 tsconfig 的应用——结构与 dev 流程差异小，保留现状（不做结构对齐，避免无效改动）。

## 验证

- `cargo build` + `cargo build --features embedded`（embedded_assets 仅在 feature 下编译）✅ 通过（残留 warnings 均为预存在项）
- 8 个前端依次 `npm run build`（vue-tsc && vite build）✅ 全部通过
- `cargo test`（含 export_openapi 断言）✅ 3 passed

## 执行记录（2026-08-13）

- [x] #2 csv_writer 去重：`csv_sink.rs` 改用 `crate::common::csv_writer`，删除本地副本，两级 mod.rs 与注释同步
- [x] #3 config_singleton 宏：`common/config.rs` 新增 `#[macro_export]` 宏，fj200c_information / ftj1c 的 config.rs 各 30 行缩为 7 行
- [x] #4 event_broadcast 宏：`common/ws.rs` 新增宏，三个角色 mod.rs 的 `static XXX_TX` + getter 样板统一（宏内自带 doc 注释，调用处用 `//` 注释避免 unused doc 警告）
- [x] #5 `ServiceRuntime::stop_in_background`：`common/service.rs` 新增，fj200c_information / ftj1c 的 stop_service 收敛（ftj1c 顺带清理未用 `thread` import）
- [x] #6 `common::ws::verify_query_token`：三个角色 ws_handler 的 token 校验收敛，清理 3 处未用 `jwt` import
- [x] #7 style.css 共享：6 份 SHA256 全同副本（326 行）→ `packages/shared/src/style.css`，`main.ts` 改 `import "@shared/style.css"`；App.vue 抽 `@shared/template/AppShell.vue`（6 个同构应用薄封装化，fj200c_main / city3d 因角色专有逻辑保留）；公共 devDeps 5 项提升至根 package.json（city3d 保留 @types/three）
- [x] #8 `@shared/template/LedgerPanel.vue`：fw100/fw150 的 Panel.vue 138 行 → 10 行薄封装（props：roleKey / permissionKey / api，鸭子类型 LedgerRow）
- [x] #11 embedded_assets：`embed_assets!` + `embed_app_routes!` 两个宏生成 8 个 rust-embed 结构体与 24 条路由
- [x] #12 `session.rs:308` 每 200ms 的 `info!` 降为 `trace!`（去掉多余 clone）
- [x] #13 shared/index.ts 注释更新为「8 个前端应用」完整列表

### 执行中发现的偏差

- 原方案 #7 计划连 `main.ts` / `types/index.ts` 一并收敛——实际 `main.ts` 各应用 import 差异（角色专有组件/插件），`types/index.ts` 为 1 行 re-export，收益低，保留现状
- protocol_generator 的 App.vue 头部注释误写为「设备台账应用（fw100）」（历史复制遗留），顺带修正
- AppShell 中 `getAppAuthStore()` 需显式泛型（`getAppAuthStore<AppAuthStore>()`），否则 vue-tsc 报 `initAuth` 不存在
