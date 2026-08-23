# Rust Web 全栈管理系统

Rust + Axum 后端 + 4 个 Vue 3 前端应用的多角色管理系统。RBAC 角色权限、WebSocket 实时监控、OpenAPI 契约自动同步前后端类型。

## 技术栈

- **后端**：Rust · Axum 0.7 · SQLite (sqlx) · JWT · utoipa (OpenAPI)
- **前端**：Vue 3 · Vite 6 · TypeScript · Element Plus · Pinia · ECharts
- **同步**：npm workspaces · orval 根据 `openapi.json` 自动生成 TS 类型与 API 客户端

## 项目结构

```
RustWeb-Vue/
├── src/                    # Rust 后端
│   ├── main.rs             # 入口：路由 + CORS + 静态托管（embedded/dev 双模式）
│   ├── routes.rs           # 路由集中注册
│   ├── roles.rs            # 角色注册表（RBAC 唯一源，经 /api/meta/roles 暴露）
│   ├── api_docs.rs         # OpenAPI 聚合 + export_openapi 测试
│   ├── embedded_assets.rs  # single-exe 打包：--features embedded 时内嵌 4 个前端 dist
│   ├── database.rs         # 建表 + 种子账号（无 sqlx 迁移文件）
│   ├── common/             # 认证、中间件、模型、DTO、错误处理、JWT、串口封装
│   ├── admin/              # 用户管理
│   ├── fj200c_information/ # 发动机监控（串口/模拟、帧解码、CSV、WebSocket）
│   ├── fj200c_main/        # 发动机测控（ECU/ADAM/DYNO 五路串口、试验、报表、WebSocket）
│   ├── mario/              # 超级马里奥复刻游戏（Canvas 游戏 + 高分榜）
│   └── role_template/      # 新角色模块参考模板
├── frontend/               # 4 个独立 Vue 应用（见下表）
├── packages/shared/        # @rustweb/shared 共享包
│   └── src/
│       ├── roles.ts        # 前端菜单配置 + 注册表拉取/缓存（loadRoleRegistry）
│       ├── session.ts      # localStorage 会话管理
│       ├── stores/auth.ts  # Pinia 认证 Store 工厂（含 stopServices 角色线程隔离）
│       ├── api/            # axios 工厂 + custom-instance + orval 生成代码
│       └── template/       # AppNavbar / LoginPage / TemplatePanel 公共组件
├── build/vite.base.ts      # 前端 vite 共享工厂（defineAppConfig，alias/proxy/base/manualChunks）
├── openapi/openapi.json    # OpenAPI spec（工具生成，提交仓库）
├── orval.config.ts         # orval 生成配置
├── deploy.bat              # 一键部署脚本（前端并行构建 → exe 内嵌 → 组装 deploy/）
├── config-fj200c_information.ini   # 发动机监控模块配置（热加载）
└── config-fj200c_main.ini  # 发动机测控模块配置（需重启）
```

角色模块采用**两级目录**：一级目录仅保留模板骨架（`mod.rs` / `handlers.rs` / `routes.rs` / `service(s).rs`），角色专有代码位于二级目录 `src/<role>/<role>/`；一级 `mod.rs` 通过 `pub use` 再导出，外部代码路径保持 `crate::<role>::x` 不变。

## 前端应用

| 应用 | 目录 | 用途 | dev 端口 | 生产路径 |
|---|---|---|---|---|
| 发动机监控 | `frontend/fj200c_information` | 服务启停、命令下发、实时监控、可视化、CSV 数据 | 5173 | `/fj200c_information` |
| 管理后台 | `frontend/admin` | 用户管理 | 5174 | `/admin` |
| 发动机测控 | `frontend/fj200c_main` | ECU/ADAM/DYNO 三路串口测控、试验、报表 | 5179 | `/fj200c_main` |
| 超级马里奥 | `frontend/mario` | Canvas 平台跳跃游戏、高分榜 | 5182 | `/mario` |

4 个应用共享同一登录态（localStorage token），跨应用跳转自动传递；各自 `vite.config.ts` 均是一行工厂调用 `defineAppConfig({ app, port, ws }, __dirname)`（`/api` 代理到后端 :3000，WebSocket 代理仅 fj200c_information / fj200c_main 开启）。

## 角色与权限（RBAC）

| 角色 | 权限 | 前端 |
|---|---|---|
| `admin` | SystemAdmin + Users* | admin |
| `fj200c_information` | Fj200cInformationMonitor | fj200c_information |
| `fj200c_main` | Fj200cMainMonitor | fj200c_main |
| `mario` | MarioMonitor | mario |

角色注册表唯一源在后端 `src/roles.rs`（`ROLE_REGISTRY`），通过 `GET /api/meta/roles` 公开；前端运行时拉取 key/name/permissions（orval 生成 `RoleInfo` 类型），经 `packages/shared/src/roles.ts` 的 `loadRoleRegistry()` 缓存，`initAuth`/`login` 时自动加载。菜单（`MENU_CONFIG`）与 `ROLE_APP_URLS` 是纯前端 UI 概念，仍手写在 `packages/shared/src/roles.ts`。

## 快速开始

### 环境要求

- Rust 1.75+（stable toolchain）
- Node.js 18+
- SQLite 无需安装（数据库文件自动创建）

### 安装与运行

```powershell
# 1. 安装后端依赖
cargo build

# 2. 安装前端依赖（npm workspaces，根目录一次即可，勿在子目录单独安装）
npm install

# 3. 启动后端（:3000）
cargo run

# 4. 启动任意前端（各自目录内）
cd frontend/fj200c_information
npm run dev        # http://localhost:5173

# 或构建生产版本
npm run build      # vue-tsc && vite build
```

### 访问

- 开发模式：各前端 `http://localhost:<port>`，登录后自动跳转对应应用
- 生产模式：`http://localhost:3000` 根路径重定向到 `/admin`
- API 文档：`http://localhost:3000/api-docs/openapi.json`（实时 spec）

## 前后端类型同步（utoipa + orval）

后端 handler 加 `#[utoipa::path]` 注解、DTO 加 `#[derive(ToSchema)]`，运行根目录命令一键生成前端代码：

```powershell
npm run gen:api    # = cargo test export_openapi && orval
```

- `cargo test export_openapi` → 生成 `openapi/openapi.json`（内置 paths/operationId 断言防漂移：31 路径 / 37 操作）
- `orval` → 生成 `packages/shared/src/api/generated/`（按 tag 拆分的请求函数 + 类型，走统一 `customInstance`）
- 各前端 `src/api/index.ts` 用生成函数组装对象 facade（如 `fj200c_informationApi.startService(...)`），视图层调用点不变
- `openapi.json` 与 `generated/` 只由工具生成，**不手改**
- WebSocket 不进 OpenAPI：`buildWebSocketUrl` 与 WS 事件类型手写于各前端 `types.ts`
- 修改 DTO/接口后：`npm run gen:api` → vue-tsc 报错处即需更新的调用点

## API 一览

| 前缀 | 权限 | 说明 |
|---|---|---|
| `/api/auth/*` | 无需/已登录 | 登录、用户信息、登出（所有角色共用；`keep_role` 语义隔离线程） |
| `/api/meta/roles` | 公开 | 角色注册表（key/name/permissions 唯一源） |
| `/api/users/*` | SystemAdmin | 用户管理 + 系统设置（admin） |
| `/api/fj200c_information/*` | Fj200cInformationMonitor | 服务启停/命令/config.ini/CSV；WS `/api/fj200c_information/ws?token=` |
| `/api/fj200c_main/*` | Fj200cMainMonitor | 三路串口测控：服务启停/ECU 指令/config.ini/CSV 64 列录制/试验信息/报表生成；WS `/api/fj200c_main/ws?token=` |
| `/api/mario/*` | MarioMonitor | 超级马里奥游戏成绩：高分榜 `GET /api/mario/scores`、提交成绩 `POST /api/mario/scores`、全局统计 `GET /api/mario/stats` |

WebSocket 不走 JWT header（浏览器 WS 不支持自定义头），token 通过 `?token=` 查询参数，handler 内部校验。

## 配置

### 环境变量（`.env`）

| 变量 | 默认值 | 说明 |
|---|---|---|
| `PORT` | `3000` | 后端端口 |
| `DATABASE_URL` | `sqlite://rustweb.db` | SQLite 连接（文件自动创建） |
| `JWT_SECRET` | dev 有默认值 | JWT 签名密钥（**release 模式缺失会拒绝启动**） |
| `JWT_EXPIRATION` | `86400` | token 过期秒数 |
| `RUST_LOG` | `info` | 日志级别 |
| `CORS_ORIGINS` | dev 放行任意 | CORS 白名单（**release 模式缺失会拒绝启动**） |

### 模块配置（INI）

- `config-fj200c_information.ini` — 发动机监控：`[Mock] InProcess = true` 开箱即用（无需硬件）；`[ConnectionN]` 串口参数；`[CSV]` 数据记录。**修改后立即生效**（热加载）
- `config-fj200c_main.ini` — 发动机测控：`[COM] Count = 5` 五路串口 ECU/Adam4015/Adam4117/Dyno/Flux；`[MOCK] SimulationMenu = true` 模拟运行；`[REPORT] StatePoints` 报表状态点。**修改后需重启服务**

## 构建与部署（Windows 一键）

```powershell
deploy.bat   # 1) 4 个前端并行 npm run build（build-frontends.ps1）2) cargo build --release --features embedded 3) 组装 deploy/
```

顺序不可颠倒：前端 dist 在**编译期**内嵌进 exe，必须先构建前端再编译后端。

产物结构（单 exe + 运行期必需文件）：

```
deploy/
├── rust-web-backend.exe     # 单文件后端（内嵌 4 个前端 dist，双击即可启动）
├── .env                     # 运行时配置（不存在时自动生成）
├── config-fj200c_information.ini   # 发动机监控模块配置（随部署自动生成）
├── config-fj200c_main.ini   # 发动机测控模块配置（随部署自动生成）
└── csv/                     # CSV 数据目录
```

- 访问：`http://localhost:3000`（4 个应用 SPA 深链接自动回退 index.html）
- 数据迁移：拷贝 `deploy/rustweb.db` 即可
- 服务绑定 `127.0.0.1`，如需外网访问修改 `src/main.rs` 为 `0.0.0.0` 后重新部署

## 数据库

建表与种子数据由 `src/database.rs` 内建（无 sqlx 迁移文件）：首次启动自动创建 `users` / `user_settings` 等表并插入 4 个种子账号（admin、fj200c_information、fj200c_main、mario，邮箱 `@7304.com`）。**种子账号初始密码是随机生成的**，明文只存 `seed_passwords` 表，经 `GET /admin/pwd` 查询（admin 后端可在用户管理设置中停用该端点）。

数据文件为运行目录下的 `rustweb.db`（开发在项目根，部署在 `deploy/`）。

## 开发指南

### 新增角色（前后端 + 生成）

1. `src/common/models.rs` 加 `Permission::XxxMonitor`；`src/roles.rs` 注册 `RoleDef`（key/name/permissions）
2. 复制 `src/role_template/` 为 `src/xxx/`（一级骨架），角色专有子模块放入二级目录 `src/xxx/xxx/`，一级 `mod.rs` 用 `pub use` 再导出，`routes.rs` 挂载 `/api/xxx/*`（用 `permission_middleware`）
3. handler 加 `#[utoipa::path]`（tags="xxx"）；`src/api_docs.rs` 追加 paths/schemas/tags，同步更新 `export_openapi` 测试里的路径/操作数量断言（当前 31 路径 / 37 操作）
4. `packages/shared/src/roles.ts` 的 `MENU_CONFIG` 加菜单、`ROLE_APP_URLS` 加应用地址（注册表数据无需手写，由 `/api/meta/roles` 同步）
5. 复制现有前端为新应用 `frontend/xxx/`：改端口/base/workspaces，角色专有文件放 `src/xxx/` 二级目录，`api/index.ts` 组装 facade，`setApiInstance` 注入
6. `npm run gen:api` 生成 `generated/api/xxx.ts`，前端 `npm run build` 通过
7. `deploy.bat` 加构建步骤；`main.rs` 加静态托管（dev 模式）+ `embedded_assets.rs` 加嵌入结构体与 `embedded_router()` 路由

### 注意事项

- 依赖只在根目录执行一次 `npm install`（workspaces 统一安装），子目录单独安装会产生重复依赖实例（曾导致 pinia 双实例黑屏）
- 前端 `npm run build` 必须在各自 `frontend/*` 子目录执行
- `Cargo.lock` 与 `package-lock.json` 均需提交（锁定依赖版本）
- 改名/新增前端应用时，`main.rs` 静态托管、`embedded_assets.rs`、`deploy.bat`、`package.json` workspaces、`vite.config.ts` base/port 都要同步改
- release 构建缺 `JWT_SECRET` / `CORS_ORIGINS` 直接拒绝启动，本地验证 release 行为需先在 `.env` 补齐

## 许可证

MIT License
