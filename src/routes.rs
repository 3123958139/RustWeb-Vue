//! # 路由注册模块
//!
//! 本模块负责集中注册所有 API 路由，采用**分层架构**：
//!
//! ```text
//! /api/auth/*    → 认证模块（所有角色共用）
//! /api/users/*   → 管理员模块（用户管理）
//! /api/fj200c_information/*  → fj200c_information 模块（发动机监控）
//! /api/fw100/*  → fw100 模块（设备台账）
//! /api/ftj1c/*   → ftj1c 模块（UDP 通信监控）
//! /api/city3d/*  → city3d 模块（城市 3D 展示）
//! /api/qgc/*     → qgc 模块（飞控地面站）
//! ```
//!
//! # 设计理念
//!
//! 1. **角色隔离**：每个角色有自己的路由前缀和中间件保护
//! 2. **单一职责**：每个模块只处理自己的业务
//! 3. **依赖注入**：数据库连接池通过 `with_state` 注入到所有处理器
//!
//! # 中间件保护
//!
//! - `auth_routes`：无需特殊权限（登录、获取用户信息）
//! - `admin_routes`：需要 `SystemAdmin` 权限（`role_middleware`）
//! - `fj200c_information_routes`：需要 `Fj200cInformationMonitor` 权限（`permission_middleware`）
//! - `fw100_routes`：需要 `Fw100Monitor` 权限
//! - `ftj1c_routes`：需要 `Ftj1cMonitor` 权限
//! - `city3d_routes`：需要 `City3dView` 权限
//! - `qgc_routes`：需要 `QgcMonitor` 权限

use crate::database::DatabaseConnection;
// 数据库连接池类型
use axum::{routing::get, Router};
// Axum 路由相关类型

/// 创建并配置所有 API 路由
///
/// # 参数
/// - `db`: 数据库连接池（`SqlitePool`），会被克隆并注入到所有处理器
///
/// # 返回值
/// 返回配置好的 `Router`，包含所有路由和中间件
///
/// # 路由表
///
/// | 路径 | 方法 | 说明 | 权限要求 |
/// |------|------|------|----------|
/// | `/health` | GET | 健康检查 | 无 |
/// | `/api/auth/login` | POST | 用户登录 | 无 |
/// | `/api/auth/profile` | GET | 获取用户信息 | 已登录 |
/// | `/api/users` | GET | 用户列表 | SystemAdmin |
/// | `/api/users` | POST | 创建用户 | SystemAdmin |
/// | `/api/fj200c_information/*` | * | 发动机监控 | Fj200cInformationMonitor |
/// | `/api/fw100/*` | * | 设备台账 | Fw100Monitor |
/// | `/api/ftj1c/*` | * | UDP 通信监控 | Ftj1cMonitor |
/// | `/api/city3d/*` | * | 城市 3D 展示 | City3dView |
pub fn create_router(db: DatabaseConnection) -> Router {
    // ============ 1. 创建各模块的子路由 ============
    //
    // `db.clone()` 克隆连接池（SQLx 的连接池是引用计数的，克隆成本很低）
    // 每个模块的 `*_router()` 函数返回该模块的路由配置
    //
    // 认证路由（所有角色共用）：
    // - `POST /api/auth/login` - 用户登录，返回 JWT token
    // - `GET /api/auth/profile` - 获取当前用户信息
    let auth_routes = crate::common::auth::routes::auth_router(db.clone());

    // 管理员路由（需要 SystemAdmin 权限）：
    // - `GET /api/users` - 获取用户列表
    // - `POST /api/users` - 创建新用户
    // - `PUT /api/users/:id` - 更新用户
    // - `DELETE /api/users/:id` - 删除用户
    let admin_routes = crate::admin::routes::admin_router(db.clone());

    // fj200c_information 角色路由（需要 Fj200cInformationMonitor 权限）：
    // - 发动机监控相关 API
    // - WebSocket 推送（实时数据）
    let fj200c_information_routes = crate::fj200c_information::routes::fj200c_information_router(db.clone());

    // fj200c_main 角色路由（需要 Fj200cMainMonitor 权限）：
    // - 发动机测控相关 API（ECU/Adam4015/Adam4117/Dyno/Flux 五路串口）
    // - WebSocket 推送（实时数据）
    let fj200c_main_routes = crate::fj200c_main::routes::fj200c_main_router(db.clone());

    // fw100 角色路由（需要 Fw100Monitor 权限）：
    // - 设备台账管理 API
    let fw100_routes = crate::fw100::routes::fw100_router(db.clone());

    // ftj1c 角色路由（需要 Ftj1cMonitor 权限）：
    // - UDP 组播通信监控 API
    // - WebSocket 推送（实时数据）
    let ftj1c_routes = crate::ftj1c::routes::ftj1c_router(db.clone());

    // city3d 角色路由（需要 City3dView 权限）：
    // - 城市区域 / 建筑 / 事件管理 API
    // - 城市概览聚合统计（3D 场景 HUD 数据源）
    let city3d_routes = crate::city3d::routes::city3d_router(db.clone());
    let fw150_routes = crate::fw150::routes::fw150_router(db.clone());
    // protocol_generator 角色路由（需要 ProtocolGeneratorMonitor 权限）：
    // - 通信协议生成（Markdown / Excel / CSV 参数表）
    let protocol_generator_routes = crate::protocol_generator::routes::protocol_generator_router(db.clone());

    // qgc 角色路由（需要 QgcMonitor 权限）：
    // - 飞控地面站：遥测监控 / 命令 / 模式 / 任务规划
    // - WebSocket 推送（telemetry / mission_progress / command_ack）
    let qgc_routes = crate::qgc::routes::qgc_router(db.clone());

    // ============ 2. 组装路由树 ============
    //
    // `Router::new()` 创建空路由
    // `.route(path, method_router)` 添加单个路由
    // `.nest(prefix, sub_router)` 嵌套子路由（自动添加前缀）
    // `.with_state(state)` 注入共享状态（数据库连接池）
    //
    // Axum 的路由系统是树形结构：
    // - 每个 `.route()` 或 `.nest()` 返回新的 `Router`（不可变链式调用）
    // - 状态通过泛型传递，类型安全
    Router::<DatabaseConnection>::new()
        // 健康检查端点：GET /health
        // 返回 200 OK，用于负载均衡器、监控系统检测服务是否正常
        .route("/health", get(crate::common::health_check))
        // OpenAPI 规范：GET /api-docs/openapi.json
        .route(
            "/api-docs/openapi.json",
            get(crate::api_docs::openapi_json),
        )
        // 角色注册表元信息：GET /api/meta/roles
        // 公开端点：前端启动时拉取 key/name/permissions（注册表唯一来源）
        .route("/api/meta/roles", get(crate::roles::list_roles))
        // 种子账号初始密码查询：GET /admin/pwd
        // 种子账号随机初始密码不打印日志，明文存 seed_passwords 表，经此端点查询
        // （显式静态路由优先于 /admin 静态托管通配，不冲突）
        .route(
            "/admin/pwd",
            get(crate::admin::handlers::list_seed_passwords),
        )
        // 嵌套路由：/api/auth/*
        // 例如：`auth_router` 中的 `/login` 会变成 `/api/auth/login`
        .nest("/api/auth", auth_routes)
        // 嵌套路由：/api/users/*
        // 管理员模块的用户管理 API
        .nest("/api/users", admin_routes)
        // 嵌套路由：/api/fj200c_information/*
        // fj200c_information 角色的发动机监控 API
        .nest("/api/fj200c_information", fj200c_information_routes)
        // 嵌套路由：/api/fj200c_main/*
        // fj200c_main 角色的发动机测控 API（ECU/Adam4015/Adam4117/Dyno/Flux 五路串口）
        .nest("/api/fj200c_main", fj200c_main_routes)
        // 嵌套路由：/api/fw100/*
        // fw100 角色的设备台账 API
        .nest("/api/fw100", fw100_routes)
        // 嵌套路由：/api/ftj1c/*
        // ftj1c 角色的 UDP 通信监控 API
        .nest("/api/ftj1c", ftj1c_routes)
        // 嵌套路由：/api/city3d/*
        // city3d 角色的城市 3D 展示 API
        .nest("/api/city3d", city3d_routes)
        // 注入数据库连接池到所有处理器
        // 处理器通过 `State(db): State<DatabaseConnection>` 提取
        .nest("/api/fw150", fw150_routes)
        .nest("/api/protocol_generator", protocol_generator_routes)
        .nest("/api/qgc", qgc_routes)
        .with_state(db)
}
