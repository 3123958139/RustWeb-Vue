# AGENTS.md

## 项目概述

Rust + Axum 后端 + 10 个 Vue 3 前端应用（9 用户端 + admin 管理后台）的全栈管理系统。不是 Tauri 项目。根目录为 npm workspaces，后端 Rust 独立构建。

## 架构

- **后端** `src/` — Axum 0.7 Web 框架，SQLite (sqlx)，JWT 认证，RBAC 角色权限
- **前端** `frontend/` — 10 个独立 Vue 3 + Vite 6 + Element Plus + Pinia 应用，各占不同端口
- **共享包** `packages/shared/` — `@rustweb/shared`，npm workspaces 引用，通过 `@shared` alias 使用
- **建表与种子数据** — 由 `src/database.rs` 内建表 + 插入种子账号（无 sqlx 迁移文件）
- **类型同步** — utoipa + OpenAPI + orval（已替代旧 ts-rs 方案）

## 前端应用与端口

| 目录 | 用途 | dev 端口 | prod 路径 |
|---|---|---|---|
| `frontend/fj200c_information` | 发动机监控 | 5173 | `/fj200c_information` |
| `frontend/admin` | 管理后台 | 5174 | `/admin` |
| `frontend/fw100` | 设备台账 | 5175 | `/fw100` |
| `frontend/ftj1c` | UDP 通信监控 | 5176 | `/ftj1c` |
| `frontend/city3d` | 城市 3D 展示 | 5177 | `/city3d` |
| `frontend/fw150` | 设备台账 | 5178 | `/fw150` |
| `frontend/fj200c_main` | 发动机测控（ECU/ADAM/DYNO 三路串口） | 5179 | `/fj200c_main` |
| `frontend/protocol_generator` | 通信协议生成 | 5180 | `/protocol_generator` |
| `frontend/qgc` | 飞控地面站（MAVLink） | 5181 | `/qgc` |
| `frontend/mario` | 超级马里奥复刻游戏 | 5182 | `/mario` |

所有前端 `vite.config.ts` 都是一行工厂调用：`defineAppConfig({ app, port, ws }, __dirname)`（共享工厂在 `build/vite.base.ts`，`@`/`@shared` alias、`/api` 代理到 `http://localhost:3000`、`base`、manualChunks 都在其中）。ws 仅 fj200c_information/fj200c_main/ftj1c/qgc 开启。新增应用必须显式传 `__dirname`（工厂内 `__dirname` 指向 build/ 而非应用目录）。

## 角色与权限（RBAC）

角色注册表**唯一源在后端** `src/roles.rs`（`ROLE_REGISTRY`），通过 `GET /api/meta/roles` 公开；前端运行时从该接口拉取 key/name/permissions（orval 生成 `RoleInfo` 类型与请求函数，经 `packages/shared/src/roles.ts` 的 `loadRoleRegistry()` 缓存，`initAuth`/`login` 时自动加载）。**菜单**（`MENU_CONFIG`）与 `ROLE_APP_URLS` 是纯前端 UI 概念，仍手写在 `packages/shared/src/roles.ts`。`Permission` 枚举：后端 `src/common/models.rs` + 前端 `packages/shared/src/types.ts` re-export 自 generated。

| 角色 | 权限 | 对应前端 |
|---|---|---|
| `admin` | SystemAdmin + Users* | admin |
| `fj200c_information` | Fj200cInformationMonitor | fj200c_information |
| `fj200c_main` | Fj200cMainMonitor | fj200c_main |
| `fw100` | Fw100Monitor | fw100 |
| `fw150` | Fw150Monitor | fw150 |
| `ftj1c` | Ftj1cMonitor | ftj1c |
| `city3d` | City3dView | city3d |
| `protocol_generator` | ProtocolGeneratorMonitor | protocol_generator |
| `qgc` | QgcMonitor | qgc |
| `mario` | MarioMonitor | mario |

新增角色：`src/roles.rs` 注册表加一项 → `npm run gen:api` 同步前端类型 → `roles.ts` 的 `MENU_CONFIG`/`ROLE_APP_URLS` 加菜单与地址 → 复制 `src/role_template/` 为后端模块 → 复制现有前端为新应用 → 见下文「新增角色」章节。

## 开发命令

```powershell
# 后端（项目根目录）
cargo run                    # 启动 Axum 后端 :3000

# 前端（在对应 frontend/* 子目录执行，依赖在根目录 npm install 一次即可）
npm run dev                  # Vite dev server
npm run build                # vue-tsc && vite build

# 类型与 API 同步（根目录）
npm run gen:api              # = cargo test export_openapi && orval

# 一键部署（Windows，根目录）
deploy.bat                   # 10 个前端并行 npm run build（build-frontends.ps1）→ cargo build --release --features embedded（前端嵌入 exe）→ 组装 deploy/ 并启动
```

## 前后端类型同步（utoipa + orval）

已替换旧的 ts-rs 方案，流程：

1. 后端 handler 加 `#[utoipa::path]` 注解（path / tags / operation_id / request_body / responses），DTO 加 `#[derive(ToSchema)]`
2. `src/api_docs.rs` 集中聚合 paths/schemas/tags
3. 根目录运行 `npm run gen:api`：
   - `cargo test export_openapi` → 生成 `openapi/openapi.json`（内置 paths/operationId 断言防漂移）
   - `orval` → 生成 `packages/shared/src/api/generated/`（`api/<tag>.ts` 请求函数 + `model/*.ts` 类型，tags-split 模式，走 `customInstance`）
4. 生成文件提交仓库，前端从 `@shared/api/generated`（或各应用 `api/` facade）导入

约定：

- `openapi.json` 与 `generated/` 只由工具生成，**不手改**
- 各前端 `src/api/index.ts` 用生成函数组装对象 facade（如 `fj200c_informationApi = { startService: fj200c_informationStartService, ... }`），视图层调用点不变
- 前端同样两级目录：一级 `src/` 仅保留模板骨架（`App.vue` / `main.ts` / `router/` / `stores/` / `types/` / `utils/` / `views/Login.vue` / `api/index.ts`），角色专有文件（views / api / components / composables 等）位于二级目录 `src/<role>/`
- WebSocket 不进 OpenAPI：`buildWebSocketUrl` 与 WS 事件类型手写于各前端 `types.ts`
- 修改 DTO/接口后：`npm run gen:api` → vue-tsc 报错处即需更新的调用点

## 后端模块结构

角色模块统一采用**两级目录**：一级目录仅保留模板骨架（`mod.rs` / `handlers.rs` / `routes.rs` / `service(s).rs`），其余角色专有 rs 文件位于二级目录 `src/<role>/<role>/`（内含 `mod.rs` 声明子模块）；一级 `mod.rs` 通过 `pub use <role>::{...};` 再导出，外部代码路径保持 `crate::<role>::x` 不变。非 rs 文件（`config-*.ini` 样本、`help_doc.md`）留在一级目录（`help_doc.md` 被 `include_str!` 相对引用）。`admin` / `fw100` / `fw150` 只有模板四件套，无二级目录。

- `src/common/` — 认证（auth/）、中间件、模型、DTO、错误处理、JWT
- `src/admin/` — 用户管理（handler + service）
- `src/fj200c_information/` — 发动机监控：二级目录含串口/模拟双数据源（`com.rs` + `mock*.rs`）、帧提取/解码、CSV 记录，WebSocket 广播
- `src/fj200c_main/` — 发动机测控：二级目录含 ECU/ADAM/DYNO 三路串口（`com.rs` + `abstract_com.rs` + `mock.rs`）、帧提取/解码、CSV 64 列记录、试验信息/报表生成，WebSocket 广播
- `src/ftj1c/` — UDP 组播通信监控：二级目录含 UDP 接收（`udp.rs`）、帧提取，WebSocket 广播
- `src/fw100/` / `src/fw150/` — 设备台账（handler + service）
- `src/city3d/` — 城市区域/建筑/事件管理 + 概览聚合统计（二级目录含 `models.rs`）
- `src/protocol_generator/` — 通信协议生成（从 demo-protocol 迁移）：参数表 CSV 读写/解析、协议 C# 代码生成、Excel/Markdown 导出（二级目录含 `generator.rs`）
- `src/qgc/` — 飞控地面站（MAVLink v2 + 模拟飞控 + 地图瓦片代理）：二级目录含 mavlink/udp/simulator/mission/process、地图瓦片代理与磁盘缓存（`tiles.rs`，离线保存/加载），WebSocket 广播
- `src/role_template/` — 新角色模块参考模板
- `src/embedded_assets.rs` — 单 exe 打包：`--features embedded` 时用 rust-embed 将 10 个前端 dist 编译期内嵌，内存服务静态资源
- `src/routes.rs` — 路由集中注册；`src/roles.rs` — 角色注册表；`src/api_docs.rs` — OpenAPI 聚合

## API 路由一览

| 前缀 | 权限要求 | 说明 |
|---|---|---|
| `/api/auth/*` | 无需/已登录 | 登录、用户信息、登出（所有角色共用）。登出 `keep_role` 语义：`POST /api/auth/logout` 带 `keep_role` 时只停其他角色的后台线程；各角色 `start_service` 自动 `stop_all_services_except(自身)`（`src/common/service.rs`），保证同一时刻只有当前角色持有串口/UDP 线程 |
| `/api/meta/roles` | 公开 | 角色注册表（key/name/permissions 唯一源） |
| `/api/users/*` | SystemAdmin | 用户管理 + 系统设置（admin）；`GET/PUT /api/users/settings/pwd-route` 停用开关控制 `/admin/pwd` |
| `/api/fj200c_information/*` | Fj200cInformationMonitor | 服务启停/命令/config.ini/CSV；`WS /api/fj200c_information/ws?token=` |
| `/api/fj200c_main/*` | Fj200cMainMonitor | 三路串口测控：服务启停/ECU 指令/config.ini/CSV 64 列录制/试验信息/报表生成/主题/模拟；`WS /api/fj200c_main/ws?token=` |
| `/api/fw100/*` | Fw100Monitor | 设备台账 |
| `/api/fw150/*` | Fw150Monitor | 设备台账 |
| `/api/ftj1c/*` | Ftj1cMonitor | 服务启停/IP 配置/config.ini；`WS /api/ftj1c/ws?token=` |
| `/api/city3d/*` | City3dView | 建筑/区域/事件/overview |
| `/api/protocol_generator/*` | ProtocolGeneratorMonitor | 通信协议生成：参数表 CSV 读写/解析、协议 C# 代码生成、Excel/Markdown 导出 |
| `/api/qgc/*` | QgcMonitor（瓦片端点 `?token=`） | 服务启停/遥测/命令/模式/任务规划/帮助；**地图瓦片代理 + 磁盘缓存**：`GET /api/qgc/tiles/{z}/{x}/{y}?token=`（命中 `tiles/` 缓存直接返回，未命中从 `[Tiles] Url` 瓦片源下载落盘，离线保存/加载）、`GET /tiles/stats`、`POST /tiles/clear`（Cesium 图片加载器无法带 Bearer 头，瓦片端点走 `?token=` 同 WS）；`WS /api/qgc/ws?token=` |
| `/api/mario/*` | MarioMonitor | 超级马里奥游戏成绩：高分榜 `GET /api/mario/scores`、提交成绩 `POST /api/mario/scores`、全局统计 `GET /api/mario/stats`（游戏逻辑运行于前端 Canvas，后端只做成绩持久化） |
| `/api-docs/openapi.json` | 公开 | 实时 OpenAPI spec（配合 Swagger UI） |

WebSocket 不走 JWT header（浏览器 WS 不支持自定义头），token 通过 `?token=` 查询参数，handler 内部校验。

## 配置与数据

- **环境变量**：`.env` + dotenv：`PORT`（默认 3000）、`DATABASE_URL`（默认 `sqlite://rustweb.db`）、`JWT_SECRET`、`JWT_EXPIRATION`、`RUST_LOG`、`CORS_ORIGINS`。**release 模式（非 debug_assertions）下 `JWT_SECRET` 与 `CORS_ORIGINS` 缺失会拒绝启动**（`src/common/jwt.rs:init`、`main.rs` CORS 白名单），dev 模式有默认值/放行任意来源
- **数据库**：SQLite 文件在运行目录自动创建（开发为根目录 `rustweb.db`，部署为 `deploy/rustweb.db`），无手动安装。建表与种子账号由 `src/database.rs` 内建（无 sqlx 迁移文件）；**种子账号初始密码是随机生成的**（10 个角色，邮箱 `@7304.com`），明文只存 `seed_passwords` 表，经 `GET /admin/pwd` 查询（`src/admin/routes.rs:73` 的 `PUT /api/users/settings/pwd-route` 可停用该端点）
- **发动机模块**：`config-fj200c_information.ini`（`[Mock] InProcess = true` 开箱即用无需硬件；`[ConnectionN]` 串口；`[CSV]` 记录），**修改后立即生效**（服务运行时热加载）
- **发动机测控模块**：`config-fj200c_main.ini`（`[COM] Count = 5` 五路串口 ECU/Adam4015/Adam4117/Dyno/Flux；`[MOCK] SimulationMenu = true` 模拟运行；`[REPORT] StatePoints` 报表状态点；`[CSV] Dir = csv`），**修改后需重启**
- **通信模块**：`config-ftj1c.ini`（`[Udp] Mock = true`；`[IP]` 16 路组播地址），**修改后需重启**
- **飞控地面站模块**：`config-qgc.ini`（`[Udp] Mock = true` 模拟飞控；`[Tiles] Url` 地图瓦片源模板，**修改后需重启**；瓦片经代理下载缓存到运行目录 `tiles/`，离线/内网直接从缓存加载）
- **静态托管**（生产）：后端 `main.rs` 双模式——`embedded` feature 下 10 个前端已内嵌进 exe（`embedded_assets.rs` 内存服务，SPA 深链接回退 index.html）；默认 dev 模式仍读磁盘 `dist-*/` 目录；根路径 `/` 重定向到 `/admin`
- **服务地址绑定**：`main.rs` 绑定 `127.0.0.1`，如需外网访问改为 `0.0.0.0`

## 注释规范

全项目注释统一使用**中文**，风格分层如下：

- **模块级文档**：Rust 文件头用 `//!`（内部以 `# 章节` 分段：架构说明/设计理念/导出说明等，如 `src/roles.rs`、`src/common/models.rs`）；TS 文件头用 `/** @module ... @description ... */` JSDoc（如 `packages/shared/src/roles.ts`）
- **公共 item 文档**：Rust 用 `///`（`# 参数` / `# 返回值` / `# 示例` / `# 语法说明` 小节）；TS 用 JSDoc `/** ... */`。DTO 字段的 `///` 注释会被 utoipa 带入 openapi.json 成为 schema description，编写时按最终用户文档对待
- **章节分隔**：`// ============ 章节名 ============` 用于分组 import 区、权限枚举、菜单配置、api_docs.rs 的 paths 注册等
- **教学性 vs 业务性注释**：一级公共模块（common/、roles.rs、role_template/）承担新人向导角色，会对 Rust 语法（Option、闭包、derive、生命周期等）写「语法说明」；二级目录角色业务代码注释只写「做什么/为什么」，不解释语法
- **数值类注释必须与实现一致**：防漂移。典型为 `src/api_docs.rs` 的 `export_openapi` 测试断言注释（当前 68 路径 / 86 操作，AGENTS.md 与 README.md 中同样数字需同步维护）与 `src/roles.rs` 模块头的角色列表——改注册表/新增角色后必须一并更新
- **生成代码不手写注释**：`openapi.json` 与 `packages/shared/src/api/generated/` 由工具生成，注释一律由源头（后端 `///`）产生
- **禁止**：无意义的 `// 注释掉的死代码`、与实现不符的过时说明（发现时随手修正）

## 构建与部署

```powershell
deploy.bat        # 1) 10 个前端并行 npm run build（build-frontends.ps1）2) cargo build --release --features embedded 3) 组装 deploy/
```

顺序不可颠倒：前端 dist 在**编译期**内嵌进 exe，必须先构建前端再编译后端。

部署产物结构（单 exe + 运行期必需文件）：

```
deploy/
├── rust-web-backend.exe     # 单文件后端（内嵌 10 个前端 dist，双击即可启动）
├── .env                     # 运行时配置（不存在时自动生成）
├── config-fj200c_information.ini        # 发动机模块配置（随部署自动生成）
├── config-fj200c_main.ini   # 发动机测控模块配置（随部署自动生成）
├── config-ftj1c.ini         # 通信模块配置
├── config-qgc.ini           # 飞控地面站模块配置（含 [Tiles] Url 瓦片源）
├── csv/                     # CSV 数据目录
└── tiles/                   # qgc 地图瓦片磁盘缓存（运行期自动生成，离线保存/加载）
```

## 常见陷阱

- 前端 `npm run build` 必须在各自 `frontend/*` 子目录执行，不是项目根目录
- 依赖只在根目录执行一次 `npm install`（workspaces 统一安装），不要在子目录单独装，否则可能产生重复依赖实例（曾导致 pinia 双实例黑屏）
- 新增角色后：只需改后端 `src/roles.rs` 注册表 + `npm run gen:api`（前端 key/name/permissions 运行时从 `/api/meta/roles` 拉取，无手写副本；若注册表未加载则前端权限为空，登录流程不受影响）
- 改名/新增前端应用时，`main.rs` 静态托管、`src/embedded_assets.rs` 嵌入结构体与路由、`deploy.bat`、`package.json` workspaces、`vite.config.ts` base/port 都要同步改
- `Cargo.lock` 需提交（锁定后端依赖版本），`package-lock.json` 需提交
- release 构建（`cargo build --release`，非 embedded 也一样）缺 `JWT_SECRET` / `CORS_ORIGINS` 直接拒绝启动，本地验证 release 行为需先在 `.env` 补齐
- `config-fj200c_information.ini` 修改立即生效（热加载），`config-fj200c_main.ini` 与 `config-ftj1c.ini` 需重启服务
- 8 个用户端 + admin 共享同一登录态（localStorage token），跨应用跳转 token 自动传递

## 新增角色流程（前端 + 后端）

1. `src/common/models.rs` 加 `Permission::XxxMonitor`；`src/roles.rs` 注册 `RoleDef`（key/name/permissions）
2. 复制 `src/role_template/` 为 `src/xxx/`（一级骨架），角色专有子模块放入二级目录 `src/xxx/xxx/`，实现 handler/service，一级 `mod.rs` 用 `pub use xxx::{...};` 再导出，`routes.rs` 挂载 `/api/xxx/*`（用 `permission_middleware`）
3. handler 加 `#[utoipa::path]`（tags="xxx"）；`src/api_docs.rs` 追加 paths/schemas/tags，并同步更新 `export_openapi` 测试里的路径数量/操作数量断言（68 路径 / 86 操作，防漂移）
4. `packages/shared/src/roles.ts` 的 `MENU_CONFIG` 加菜单、`ROLE_APP_URLS` 加应用地址（注册表数据无需手写，由 `/api/meta/roles` 同步）
5. 复制现有前端为新应用 `frontend/xxx/`：改端口/base/workspaces，角色专有文件放 `src/xxx/` 二级目录，`api/index.ts` 组装 facade，`setApiInstance` 注入
6. `npm run gen:api` 生成 `generated/api/xxx.ts`，前端 `npm run build` 通过
7. `deploy.bat` 加构建步骤；`main.rs` 加静态托管（dev 模式）+ `embedded_assets.rs` 加嵌入结构体与 `embedded_router()` 路由

## 调试

- 后端：`cargo run`，`RUST_LOG` 控制日志级别
- 前端：Vite dev server + `/api` proxy；生产环境由后端托管
- 数据库：SQLite 文件在运行目录（根 `rustweb.db` / `deploy/rustweb.db`），可用任意 SQLite 客户端查看
- API 契约：浏览器访问 `http://localhost:3000/api-docs/openapi.json`
