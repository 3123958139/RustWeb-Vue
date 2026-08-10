//! # 认证与授权中间件模块
//!
//! 本模块提供三个 Axum 中间件，用于保护 API 端点：
//!
//! | 中间件 | 功能 | 用途 |
//! |--------|------|------|
//! | `auth_middleware` | 认证用户身份 | 所有需要登录的端点 |
//! | `permission_middleware` | 检查特定权限 | 普通角色面板端点 |
//! | `role_middleware` | 检查管理角色 | 管理端点（SystemAdmin） |
//!
//! # 工作流程
//!
//! ```text
//! 请求 → 提取 Authorization 头 → 解析 JWT → 查找用户 → 检查权限 → 放行/拒绝
//! ```
//!
//! # 语法说明
//!
//! Axum 中间件签名：
//! ```rust
//! async fn middleware(
//!     State(db): State<DatabaseConnection>,
//!     mut request: Request,
//!     next: Next,
//! ) -> Result<Response, StatusCode>
//! ```
//!
//! - `State(db)`: 从共享状态中提取数据库连接池
//! - `request`: HTTP 请求对象
//! - `next`: 调用下一个中间件或处理器
//! - 返回 `Ok(response)` 放行，`Err(status)` 拒绝

use crate::common::auth::services::AuthService;  // 认证服务
use crate::common::jwt;                          // JWT 工具函数
use crate::common::models::{Permission, User};   // 数据模型
use crate::database::DatabaseConnection;         // 数据库连接池类型
use axum::{
    extract::{Request, State},  // 请求提取器和状态提取器
    http::StatusCode,           // HTTP 状态码
    middleware::Next,           // 下一个中间件/处理器
    response::Response,        // HTTP 响应
};

/// 从请求头中提取并验证 JWT 令牌，返回用户 ID
///
/// # 参数
/// - `request`: HTTP 请求的引用
///
/// # 返回值
/// - `Ok(Uuid)` - 成功返回用户 ID
/// - `Err(StatusCode::UNAUTHORIZED)` - 令牌无效或缺失
///
/// # 语法说明
///
/// - `request.headers().get("Authorization")`: 获取 Authorization 头
/// - `.and_then(...)`: 链式调用，前一步为 None 时跳过
/// - `.strip_prefix("Bearer ")`: 去掉 "Bearer " 前缀
/// - `.ok_or(StatusCode::UNAUTHORIZED)`: 将 Option 转为 Result
///
/// # 安全说明
///
/// 此函数是同步的（不跨 await 持有 `&Request`），避免 `!Send` 问题。
/// Axum 的中间件要求 Future 是 `Send` 的（可跨线程发送）。
fn extract_user_id(request: &Request) -> Result<uuid::Uuid, StatusCode> {
    // 从请求头提取 Authorization: Bearer <token>
    let auth_header = request
        .headers()
        .get("Authorization")
        .and_then(|auth_header| auth_header.to_str().ok())
        .and_then(|auth_str| auth_str.strip_prefix("Bearer "));

    // 提取令牌字符串
    let token = auth_header.ok_or(StatusCode::UNAUTHORIZED)?;

    // 验证 JWT 令牌并返回用户 ID
    jwt::verify_token(token).map_err(|_| StatusCode::UNAUTHORIZED)
}

/// 根据用户 ID 从数据库加载用户
///
/// # 参数
/// - `db`: 数据库连接池
/// - `user_id`: 用户 UUID
///
/// # 返回值
/// - `Ok(User)` - 找到用户
/// - `Err(StatusCode::UNAUTHORIZED)` - 用户不存在
async fn load_user(
    db: &DatabaseConnection,
    user_id: uuid::Uuid,
) -> Result<User, StatusCode> {
    AuthService::get_user_by_id(db, user_id)
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?  // 数据库错误视为未认证
        .ok_or(StatusCode::UNAUTHORIZED)          // 用户不存在视为未认证
}

/// 认证中间件：验证用户身份（不做权限检查）
///
/// # 功能
/// 1. 从 `Authorization` 头提取 JWT 令牌
/// 2. 验证令牌有效性
/// 3. 从数据库加载用户
/// 4. 将用户信息注入请求扩展（后续处理器可通过 `Extension(user)` 提取）
///
/// # 使用场景
/// - 获取用户信息（`GET /api/auth/profile`）
/// - 所有需要登录但不需要特定权限的端点
///
/// # 语法说明
///
/// `State(db): State<DatabaseConnection>` 是 Axum 的状态提取器：
/// - 从 `Router::with_state(db)` 注入的状态中提取 `DatabaseConnection`
/// - 类似依赖注入（Dependency Injection）
///
/// `request.extensions_mut().insert(user)` 将用户存入请求扩展：
/// - 后续处理器通过 `Extension(user): Extension<User>` 提取
pub async fn auth_middleware(
    State(db): State<DatabaseConnection>,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let user_id = extract_user_id(&request)?;
    let user = load_user(&db, user_id).await?;
    request.extensions_mut().insert(user);
    Ok(next.run(request).await)
}

/// 权限验证中间件：检查用户是否拥有特定权限
///
/// # 功能
/// 在 `auth_middleware` 的基础上，额外检查用户是否拥有指定权限。
///
/// # 参数
/// - `required_permission`: 需要的权限
///
/// # 使用场景
/// - 角色面板端点（如 `GET /api/fj200c_information/data` 需要 `Fj200cInformationMonitor` 权限）
///
/// # 工作原理
///
/// 1. 验证用户身份（同 `auth_middleware`）
/// 2. 从角色注册表获取用户的权限列表
/// 3. 检查是否包含 `required_permission`
/// 4. 不包含则返回 403 Forbidden
///
/// # 语法说明
///
/// `permission_middleware` 接受一个额外的 `required_permission` 参数。
/// 在 `routes.rs` 中通过 `layer(middleware::from_fn_with_state(...))` 使用。
pub async fn permission_middleware(
    required_permission: Permission,
    State(db): State<DatabaseConnection>,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let user_id = extract_user_id(&request)?;
    let user = load_user(&db, user_id).await?;

    // 检查用户是否拥有所需权限
    if !user.has_permission(&required_permission) {
        return Err(StatusCode::FORBIDDEN);  // 403 无权限
    }

    request.extensions_mut().insert(user);
    Ok(next.run(request).await)
}

/// 管理角色中间件：检查用户是否拥有 SystemAdmin 权限
///
/// # 功能
/// 验证用户是否是管理员（拥有 `SystemAdmin` 权限）。
///
/// # 使用场景
/// - 管理端点（用户管理、角色管理等）
///
/// # 设计说明
///
/// - 拥有 `SystemAdmin` 权限的角色视为管理角色
/// - 在角色注册表中给某个角色加上 `SystemAdmin` 权限，
///   该角色的路由自动受此中间件保护
/// - 无需为每个管理端点单独配置权限
pub async fn role_middleware(
    State(db): State<DatabaseConnection>,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let user_id = extract_user_id(&request)?;
    let user = load_user(&db, user_id).await?;

    // 检查用户是否拥有 SystemAdmin 权限
    if !user.has_permission(&Permission::SystemAdmin) {
        return Err(StatusCode::FORBIDDEN);  // 403 无权限
    }

    request.extensions_mut().insert(user);
    Ok(next.run(request).await)
}
