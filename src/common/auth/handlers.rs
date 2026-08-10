//! # 认证处理器模块
//!
//! 本模块定义认证相关的 HTTP 端点处理器：
//!
//! | 端点 | 方法 | 功能 | 权限 |
//! |------|------|------|------|
//! | `/api/auth/login` | POST | 用户登录 | 无 |
//! | `/api/auth/profile` | GET | 获取当前用户信息 | 已登录 |
//!
//! # 响应格式
//!
//! 所有端点返回统一的 `ApiResponse` 格式：
//! ```json
//! {
//!     "success": true,
//!     "data": { ... },
//!     "message": null
//! }
//! ```

use crate::common::error::AppError;                     // 统一错误类型
use crate::common::jwt;                                 // JWT 工具
use crate::common::models::{ApiResponse, LoginRequest, LoginResponse, User};  // 数据模型
use crate::common::rate_limit;                          // 登录速率限制
use crate::common::auth::services::AuthService;         // 认证服务
use axum::{extract::{ConnectInfo, Extension, State}, Json};                   // Axum 提取器
use crate::database::DatabaseConnection;                // 数据库连接池
use std::net::SocketAddr;                               // 客户端地址（速率限制键）
use validator::Validate;                                 // 输入验证

/// 用户登录处理器
///
/// # 端点
/// `POST /api/auth/login`
///
/// # 请求体
/// ```json
/// {
///     "email": "user@example.com",
///     "password": "123456"
/// }
/// ```
///
/// # 成功响应
/// ```json
/// {
///     "success": true,
///     "data": {
///         "token": "eyJhbGciOiJIUzI1NiIs...",
///         "user": {
///             "id": "...",
///             "username": "admin",
///             "email": "admin@example.com",
///             "role": "admin"
///         }
///     }
/// }
/// ```
///
/// # 错误响应
/// - 400: 邮箱或密码错误
/// - 400: 请求参数验证失败
///
/// # 参数说明
///
/// - `State(db)`: 数据库连接池（从路由状态提取）
/// - `Json(login_data)`: JSON 请求体（自动反序列化为 `LoginRequest`）
///
/// # 语法说明
///
/// Axum 的处理器（handler）使用**提取器**（extractor）获取参数：
/// - `State<T>`: 从路由状态提取
/// - `Json<T>`: 从请求体提取 JSON
/// - `Extension<T>`: 从请求扩展提取（中间件注入的数据）
///
/// 返回值 `Result<Json<ApiResponse<LoginResponse>>, AppError>`：
/// - 成功：`Ok(Json(...))` - 自动序列化为 JSON 响应
/// - 失败：`Err(AppError)` - 自动转换为错误响应
#[utoipa::path(
    post,
    tag = "auth",
    path = "/api/auth/login",
    operation_id = "authLogin",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "登录成功", body = ApiResponse<LoginResponse>),
    ),
)]
pub async fn login(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(db): State<DatabaseConnection>,
    Json(login_data): Json<LoginRequest>,
) -> Result<Json<ApiResponse<LoginResponse>>, AppError> {
    // 验证输入数据（邮箱格式等）
    // `validate()` 返回 `Result<(), ValidationErrors>`
    // `?` 操作符在验证失败时提前返回错误
    login_data.validate()?;

    // 登录速率限制：按 IP + 邮箱滑动窗口（5 次/分钟），防 bcrypt CPU DoS 与暴力破解
    let limit_key = format!("{}:{}", addr.ip(), login_data.email);
    if let Err(wait_secs) = rate_limit::check_and_record(&limit_key) {
        return Err(AppError::too_many_requests(format!(
            "尝试次数过多，请 {} 秒后重试",
            wait_secs
        )));
    }

    // 调用认证服务验证邮箱和密码
    // `map_err()` 将服务层错误转换为 `AppError`
    let user = AuthService::login(&db, login_data)
        .await
        .map_err(|e| AppError::bad_request(e.to_string()))?;

    // 登录成功，清除失败记录
    rate_limit::clear(&limit_key);

    // 创建 JWT 令牌
    let token = jwt::create_token(&user)?;

    // 构建响应
    let response = LoginResponse { token, user };
    Ok(Json(ApiResponse::success(response)))
}

/// 获取当前用户信息
///
/// # 端点
/// `GET /api/auth/profile`
///
/// # 权限
/// 需要登录（`auth_middleware` 保护）
///
/// # 成功响应
/// ```json
/// {
///     "success": true,
///     "data": {
///         "id": "...",
///         "username": "admin",
///         "email": "admin@example.com",
///         "role": "admin"
///     }
/// }
/// ```
///
/// # 参数说明
///
/// `Extension(user)`: 从请求扩展提取用户信息
/// 用户由 `auth_middleware` 中间件注入到请求扩展
///
/// # 语法说明
///
/// `Extension<T>` 是 Axum 的扩展提取器：
/// - 中间件通过 `request.extensions_mut().insert(user)` 注入数据
/// - 处理器通过 `Extension(user): Extension<User>` 提取数据
/// - 类似于"请求上下文"，用于在中间件和处理器之间传递数据
#[utoipa::path(
    tag = "auth",
    get,
    path = "/api/auth/profile",
    operation_id = "authGetProfile",
    responses(
        (status = 200, description = "当前用户信息", body = ApiResponse<User>),
    ),
)]
pub async fn get_profile(
    Extension(user): Extension<User>,
) -> Result<Json<ApiResponse<User>>, AppError> {
    Ok(Json(ApiResponse::success(user)))
}
