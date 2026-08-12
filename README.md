# Rust Web 全栈管理系统

Rust + Axum 后端 + 6 个 Vue 3 前端应用的多角色管理系统。RBAC 角色权限、WebSocket 实时监控、OpenAPI 契约自动同步前后端类型。

## 技术栈

- **后端**：Rust · Axum 0.7 · SQLite (sqlx) · JWT · utoipa (OpenAPI)
- **前端**：Vue 3 · Vite 6 · TypeScript · Element Plus · Pinia · ECharts · three.js (city3d)
- **同步**：npm workspaces · orval 根据 `openapi.json` 自动生成 TS 类型与 API 客户端

## 项目结构

```
RustWeb-Vue/
├── src/                    # Rust 后端
│   ├── main.rs             # 入口：路由 + CORS + 8 个 dist-*/ 静态托管
│   ├── routes.rs           # 路由集中注册
│   ├── roles.rs            # 角色注册表（RBAC 唯一源，经 /api/meta/roles 暴露）
│   ├── api_docs.rs         # OpenAPI 聚合 + export_openapi 测试
│   ├── common/             # 认证、中间件、模型、DTO、错误处理、JWT
│   ├── admin/              # 用户管理
│   ├── fj200c_information/ # 发动机监控（串口/模拟、帧解码、CSV、WebSocket）
│   ├── fj200c_main/        # 发动机测控（ECU/ADAM/DYNO 三路串口、试验、报表）
│   ├── ftj1c/              # UDP 组播通信监控
│   ├── fw100/  fw150/      # 设备台账
│   ├── city3d/             # 城市区域/建筑/事件 + 概览统计
│   ├── protocol_generator/ # 通信协议生成（参数表 CSV、协议代码、Excel/Markdown 导出）
│   └── role_template/      # 新角色模块参考模板
├── frontend/               # 8 个独立 Vue 应用（见下表）
├── packages/shared/        # @rustweb/shared 共享包
│   └── src/
│       ├── roles.ts        # 前端菜单配置 + 注册表拉取/缓存（loadRoleRegistry）
│       ├── session.ts      # localStorage 会话管理
│       ├── stores/auth.ts  # Pinia 认证 Store 工厂
│       ├── api/            # axios 工厂 + custom-instance + orval 生成代码
│       └── template/       # AppNavbar / LoginPage / TemplatePanel 公共组件
├── openapi/openapi.json    # OpenAPI spec（工具生成，提交仓库）
├── orval.config.ts         # orval 生成配置
├── deploy.bat              # 一键部署脚本
├── config-fj200c.ini       # 发动机模块配置
└── config-ftj1c.ini        # 通信模块配置
```

## 前端应用

| 应用 | 目录 | 用途 | dev 端口 | 生产路径 |
|---|---|---|---|---|
| 发动机监控 | `frontend/fj200c_information` | 服务启停、命令下发、实时监控、可视化、CSV 数据 | 5173 | `/fj200c_information` |
| 管理后台 | `frontend/admin` | 用户管理 | 5174 | `/admin` |
| 设备台账 | `frontend/fw100` | 设备台账 CRUD | 5175 | `/fw100` |
| 通信监控 | `frontend/ftj1c` | UDP 组播实时监控 | 5176 | `/ftj1c` |
| 城市 3D | `frontend/city3d` | 3D 场景展示与数据管理 | 5177 | `/city3d` |
| 设备台账 | `frontend/fw150` | 设备台账 CRUD | 5178 | `/fw150` |
| 发动机测控 | `frontend/fj200c_main` | ECU/ADAM/DYNO 三路串口测控、试验、报表 | 5179 | `/fj200c_main` |
| 通信协议生成 | `frontend/protocol_generator` | 参数表 CSV 编辑、协议 C# 代码生成、Excel/Markdown 导出 | 5180 | `/protocol_generator` |

8 个应用共享同一登录态（localStorage token），跨应用跳转自动传递；各自 `vite.config.ts` 将 `/api` 代理到后端 :3000（fj200c_information / ftj1c / fj200c_main 额外开启 WebSocket 代理）。

## 角色与权限（RBAC）

| 角色 | 权限 | 前端 |
|---|---|---|
| `admin` | SystemAdmin + Users* | admin |
| `fj200c` | Fj200cMonitor | fj200c |
| `fw100` | Fw100Monitor | fw100 |
| `fw150` | Fw150Monitor | fw150 |
| `ftj1c` | Ftj1cMonitor | ftj1c |
| `city3d` | City3dView | city3d |

角色注册表唯一源在后端 `src/roles.rs`，通过 `GET /api/meta/roles` 公开；前端运行时拉取 key/name/permissions（orval 生成 `RoleInfo` 类型），菜单等纯前端 UI 概念仍维护在 `packages/shared/src/roles.ts`。每个角色对应一个专属前端应用，登录后菜单由注册表动态生成。

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
cd frontend/fj200c
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

- `cargo test export_openapi` → 生成 `openapi/openapi.json`（内置 paths 断言防漂移）
- `orval` → 生成 `packages/shared/src/api/generated/`（按 tag 拆分的请求函数 + 类型，走统一 `customInstance`）
- 各前端 `src/api/index.ts` 用生成函数组装对象 facade（如 `fj200cApi.startService(...)`），视图层调用点不变
- `openapi.json` 与 `generated/` 只由工具生成，**不手改**
- WebSocket 不进 OpenAPI：`buildWebSocketUrl` 与 WS 事件类型手写于各前端 `types.ts`
- 修改 DTO/接口后：`npm run gen:api` → vue-tsc 报错处即需更新的调用点

## API 一览

| 前缀 | 权限 | 说明 |
|---|---|---|
| `/api/auth/*` | 无需/已登录 | 登录、用户信息（所有角色共用） |
| `/api/users/*` | SystemAdmin | 用户管理（admin） |
| `/api/fj200c/*` | Fj200cMonitor | 服务启停/命令/config.ini/CSV；WS `/api/fj200c/ws?token=` |
| `/api/fw100/*` | Fw100Monitor | 设备台账 |
| `/api/fw150/*` | Fw150Monitor | 设备台账 |
| `/api/ftj1c/*` | Ftj1cMonitor | 服务启停/IP 配置/config.ini；WS `/api/ftj1c/ws?token=` |
| `/api/city3d/*` | City3dView | 建筑/区域/事件/overview |

WebSocket 不走 JWT header（浏览器 WS 不支持自定义头），token 通过 `?token=` 查询参数，handler 内部校验。

## 配置

### 环境变量（`.env`，可选）

| 变量 | 默认值 | 说明 |
|---|---|---|
| `PORT` | `3000` | 后端端口 |
| `DATABASE_URL` | `sqlite://rustweb.db` | SQLite 连接（文件自动创建） |
| `JWT_SECRET` | `your-secret-key` | JWT 签名密钥（生产环境务必修改） |
| `JWT_EXPIRATION` | `86400` | token 过期秒数 |
| `RUST_LOG` | `info` | 日志级别 |

### 模块配置（INI）

- `config-fj200c.ini` — 发动机模块：`[Mock] InProcess = true` 开箱即用（无需硬件）；`[ConnectionN]` 串口参数；`[CSV]` 数据记录。**修改后立即生效**（热加载）
- `config-ftj1c.ini` — 通信模块：`[Udp] Mock = true`；`[IP]` 16 路组播地址。**修改后需重启服务**

## 构建与部署（Windows 一键）

```powershell
deploy.bat   # cargo build --release → 6 个前端依次 build → 组装 deploy/
```

产物结构：

```
deploy/
├── rust-web-backend.exe     # 后端可执行文件
├── start-deploy.bat         # 启动脚本（双击运行）
├── .env                     # 运行时配置（不存在时自动生成）
├── dist-fj200c/ ... dist-fw150/   # 6 个前端构建产物
├── config-fj200c.ini        # 发动机模块配置
├── config-ftj1c.ini         # 通信模块配置
└── csv/                     # CSV 数据目录
```

- 访问：`http://localhost:3000`（/fj200c、/admin、/fw100、/fw150、/ftj1c、/city3d，SPA 深链接自动回退 index.html）
- 数据迁移：拷贝 `deploy/rustweb.db` 即可
- 跨平台部署：把 `deploy/` 目录拷到目标机器双击 `start-deploy.bat`
- 服务绑定 `127.0.0.1`，如需外网访问修改 `src/main.rs` 为 `0.0.0.0` 后重新部署

## 数据库

建表与种子数据由 `src/database.rs` 内建（无 sqlx 迁移文件）：首次启动自动创建
`users` / `user_settings` 等表并插入 6 个种子账号（admin、fj200c、fw100、fw150、ftj1c、city3d，密码均 `123456`，部署后请修改）。
数据文件为运行目录下的 `rustweb.db`（开发在项目根，部署在 `deploy/`）。

## 开发指南

### 新增角色（前后端 + 生成）

1. `src/common/models.rs` 加 `Permission::XxxMonitor`；`src/roles.rs` 注册 `RoleDef`（key/name/permissions）
2. 复制 `src/role_template/` 为 `src/xxx/`，实现 handler/service，`routes.rs` 挂载 `/api/xxx/*`
3. handler 加 `#[utoipa::path]`（tags="xxx"）；`src/api_docs.rs` 追加 paths/schemas/tags
4. `packages/shared/src/roles.ts` 的 `MENU_CONFIG` 加菜单、`ROLE_APP_URLS` 加应用地址（注册表数据无需手写，由 `/api/meta/roles` 同步）
5. 复制现有前端为新应用 `frontend/xxx/`：改端口/base/workspaces，`api/index.ts` 组装 facade，`setApiInstance` 注入
6. `npm run gen:api` 生成 `generated/api/xxx.ts`，前端 `npm run build` 通过
7. `deploy.bat` 加构建步骤；`main.rs` 加静态托管

### 注意事项

- 依赖只在根目录执行一次 `npm install`（workspaces 统一安装），子目录单独安装会产生重复依赖实例（曾导致 pinia 双实例黑屏）
- 前端 `npm run build` 必须在各自 `frontend/*` 子目录执行
- `Cargo.lock` 被 `.gitignore` 忽略，`package-lock.json` 需提交
- 新增角色后两端 `key` 不一致会导致登录后权限为空（403）

## 许可证

MIT License
