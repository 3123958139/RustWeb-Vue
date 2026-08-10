//! # 管理员处理器模块
//!
//! 实现用户管理的 HTTP 端点。
//!
//! # Axum 提取器说明
//!
//! | 提取器 | 功能 | 示例 |
//! |--------|------|------|
//! | `State(db)` | 从路由状态提取数据库连接池 | 所有端点 |
//! | `Json(req)` | 从请求体提取 JSON 数据 | POST/PUT 请求 |
//! | `Path(id)` | 从 URL 路径提取参数 | `/:id` |
//! | `Extension(user)` | 从请求扩展提取用户信息 | 中间件注入 |

use crate::admin::services::UserAdminService;           // 管理服务
use crate::common::auth::services::AuthService;         // 认证服务（创建用户）
use crate::common::error::AppError;                     // 统一错误
use crate::common::models::{
    ApiResponse, CreateUserRequest, UpdateUserRoleRequest, User,  // 数据模型
};
use crate::database::DatabaseConnection;                // 数据库连接池
use crate::roles;                                       // 角色注册表
use axum::{
    extract::{Extension, Path, State},  // Axum 提取器
    Json,                               // JSON 提取/响应
};
use uuid::Uuid;                       // UUID 类型
use validator::Validate;               // 输入验证

/// 种子账号初始密码信息
#[derive(Debug, serde::Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct SeedPasswordInfo {
    /// 用户名
    pub username: String,
    /// 邮箱（种子账号被删除后为 null）
    pub email: Option<String>,
    /// 角色（种子账号被删除后为 null）
    pub role: Option<String>,
    /// 初始密码明文
    pub password: String,
    /// 密码记录创建时间
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// 初始密码查询路由停用状态
#[derive(Debug, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct PwdRouteStatus {
    /// 是否停用（true 表示 GET /admin/pwd 不再返回初始密码）
    pub disabled: bool,
}

/// 查询初始密码查询路由停用状态
///
/// # 端点
/// `GET /api/users/settings/pwd-route`
///
/// # 权限
/// SystemAdmin（role_middleware）
#[utoipa::path(
    tag = "admin",
    get,
    path = "/api/users/settings/pwd-route",
    operation_id = "adminGetPwdRouteStatus",
    responses(
        (status = 200, description = "初始密码查询路由停用状态", body = ApiResponse<PwdRouteStatus>),
    ),
)]
pub async fn get_pwd_route_status(
    State(db): State<DatabaseConnection>,
) -> Result<Json<ApiResponse<PwdRouteStatus>>, AppError> {
    let disabled = UserAdminService::is_pwd_route_disabled(&db)
        .await
        .map_err(|e| AppError::internal_error(e.to_string()))?;
    Ok(Json(ApiResponse::success(PwdRouteStatus { disabled })))
}

/// 设置初始密码查询路由停用状态
///
/// # 端点
/// `PUT /api/users/settings/pwd-route`
///
/// # 权限
/// SystemAdmin（role_middleware）
#[utoipa::path(
    tag = "admin",
    put,
    path = "/api/users/settings/pwd-route",
    operation_id = "adminSetPwdRouteStatus",
    request_body = PwdRouteStatus,
    responses(
        (status = 200, description = "设置成功", body = ApiResponse<PwdRouteStatus>),
    ),
)]
pub async fn set_pwd_route_status(
    State(db): State<DatabaseConnection>,
    Json(req): Json<PwdRouteStatus>,
) -> Result<Json<ApiResponse<PwdRouteStatus>>, AppError> {
    UserAdminService::set_pwd_route_disabled(&db, req.disabled)
        .await
        .map_err(|e| AppError::internal_error(e.to_string()))?;
    Ok(Json(ApiResponse::success(req)))
}

/// 查询种子账号初始密码
///
/// # 端点
/// `GET /admin/pwd`
///
/// # 权限
/// 无（种子账号首次登录需要此密码，无法要求先登录）
///
/// # 说明
///
/// 种子账号的随机初始密码不打印到启动日志，明文只存于 `seed_passwords`
/// 表，通过本端点查询。登录后应立即修改密码。
///
/// 管理员在管理后台勾选"停用初始密码查询"后，本端点返回 403，
/// 不再处理查询请求（相当于路由失效）。
#[utoipa::path(
    tag = "admin",
    get,
    path = "/admin/pwd",
    operation_id = "adminListSeedPasswords",
    responses(
        (status = 200, description = "种子账号初始密码列表", body = ApiResponse<Vec<SeedPasswordInfo>>),
    ),
)]
pub async fn list_seed_passwords(
    State(db): State<DatabaseConnection>,
) -> Result<Json<ApiResponse<Vec<SeedPasswordInfo>>>, AppError> {
    // 停用开关：管理员勾选后本端点不再返回初始密码（相当于无效）
    if UserAdminService::is_pwd_route_disabled(&db)
        .await
        .map_err(|e| AppError::internal_error(e.to_string()))?
    {
        return Err(AppError::forbidden("初始密码查询已停用"));
    }

    let rows = sqlx::query_as::<_, SeedPasswordInfo>(
        "SELECT sp.username, u.email, u.role, sp.password, sp.created_at
         FROM seed_passwords sp
         LEFT JOIN users u ON u.username = sp.username
         ORDER BY sp.created_at",
    )
    .fetch_all(&db)
    .await
    .map_err(|e| AppError::internal_error(e.to_string()))?;
    Ok(Json(ApiResponse::success(rows)))
}

/// 获取用户列表
///
/// # 端点
/// `GET /api/users`
///
/// # 权限
/// - `role_middleware`: 需要 SystemAdmin 权限
/// - `users_read_middleware`: 需要 UsersRead 权限
///
/// # 成功响应
/// ```json
/// {
///     "success": true,
///     "data": [
///         { "id": "...", "username": "admin", "role": "admin", ... },
///         { "id": "...", "username": "fj200c_information", "role": "fj200c_information", ... }
///     ]
/// }
/// ```
#[utoipa::path(
    tag = "admin",
    get,
    path = "/api/users",
    operation_id = "adminListUsers",
    responses(
        (status = 200, description = "用户列表", body = ApiResponse<Vec<User>>),
    ),
)]
pub async fn list_users(
    State(db): State<DatabaseConnection>,
) -> Result<Json<ApiResponse<Vec<User>>>, AppError> {
    let users = UserAdminService::list_users(&db)
        .await
        .map_err(|e| AppError::internal_error(e.to_string()))?;
    Ok(Json(ApiResponse::success(users)))
}

/// 创建用户
///
/// # 端点
/// `POST /api/users`
///
/// # 权限
/// - `role_middleware`: 需要 SystemAdmin 权限
/// - `users_write_middleware`: 需要 UsersWrite 权限
///
/// # 请求体
/// ```json
/// {
///     "username": "newuser",
///     "email": "user@example.com",
///     "password": "123456",
///     "role": "fj200c_information"   // 可选，默认 "fj200c_information"
/// }
/// ```
///
/// # 安全约束
///
/// - `role` 必须在角色注册表中（白名单校验）
/// - 默认角色为 `fj200c_information`（普通用户端角色）
#[utoipa::path(
    post,
    tag = "admin",
    path = "/api/users",
    operation_id = "adminCreateUser",
    request_body = CreateUserRequest,
    responses(
        (status = 200, description = "创建成功", body = ApiResponse<User>),
    ),
)]
pub async fn create_user(
    State(db): State<DatabaseConnection>,
    Json(req): Json<CreateUserRequest>,
) -> Result<Json<ApiResponse<User>>, AppError> {
    // 验证输入（用户名长度、邮箱格式、密码长度）
    req.validate()?;

    // 角色绑定白名单：只允许注册表中已登记的角色
    // `unwrap_or_else` 在 role 为 None 时提供默认值
    let role = req.role.unwrap_or_else(|| "fj200c_information".to_string());
    if !roles::is_registered_role(&role) {
        return Err(AppError::bad_request(format!("未知角色: {}", role)));
    }

    // 调用认证服务创建用户（唯一入口）
    let user = AuthService::create_user(&db, &req.username, &req.email, &req.password, &role)
        .await
        .map_err(|e| AppError::bad_request(e.to_string()))?;

    Ok(Json(ApiResponse::success(user)))
}

/// 修改用户角色
///
/// # 端点
/// `PUT /api/users/:id/role`
///
/// # 权限
/// - `role_middleware`: 需要 SystemAdmin 权限
/// - `users_write_middleware`: 需要 UsersWrite 权限
///
/// # 请求体
/// ```json
/// { "role": "admin" }
/// ```
///
/// # 安全约束
///
/// - 角色必须在注册表中
/// - 不能移除自己的管理角色（防止管理域无人可用）
///
/// # 语法说明
///
/// - `Path(user_id): Path<Uuid>`: 从 URL 提取 `:id` 参数并解析为 UUID
/// - `Extension(current_user)`: 从请求扩展提取当前登录用户（中间件注入）
#[utoipa::path(
    put,
    tag = "admin",
    path = "/api/users/{id}/role",
    operation_id = "adminUpdateUserRole",
    request_body = UpdateUserRoleRequest,
    params(
        ("id" = Uuid, Path, description = "用户 UUID"),
    ),
    responses(
        (status = 200, description = "修改成功", body = ApiResponse<User>),
    ),
)]
pub async fn update_user_role(
    State(db): State<DatabaseConnection>,
    Extension(current_user): Extension<User>,
    Path(user_id): Path<Uuid>,
    Json(req): Json<UpdateUserRoleRequest>,
) -> Result<Json<ApiResponse<User>>, AppError> {
    // 角色绑定白名单校验
    if !roles::is_registered_role(&req.role) {
        return Err(AppError::bad_request(format!("未知角色: {}", req.role)));
    }

    // 安全约束：不能移除自己的管理角色
    // 如果目标用户是自己，且新角色没有 SystemAdmin 权限，则拒绝
    if current_user.id == user_id && !roles::permissions_for(&req.role).contains(&crate::common::models::Permission::SystemAdmin) {
        return Err(AppError::bad_request("不能移除自己的管理角色"));
    }

    // 更新用户角色
    let user = UserAdminService::update_user_role(&db, user_id, &req.role)
        .await
        .map_err(|e| AppError::internal_error(e.to_string()))?
        .ok_or_else(|| AppError::not_found("用户不存在"))?;

    Ok(Json(ApiResponse::success(user)))
}

/// 删除用户
///
/// # 端点
/// `DELETE /api/users/:id`
///
/// # 权限
/// - `role_middleware`: 需要 SystemAdmin 权限
/// - `users_delete_middleware`: 需要 UsersDelete 权限
///
/// # 安全约束
///
/// - 不能删除自己（防止误操作导致管理域无人可用）
///
/// # 语法说明
///
/// `Result<Json<ApiResponse<()>>, AppError>`:
/// - `()` 是 Rust 的单元类型（unit type），表示"无数据"
/// - 删除成功后返回空数据
#[utoipa::path(
    tag = "admin",
    delete,
    path = "/api/users/{id}",
    operation_id = "adminDeleteUser",
    params(
        ("id" = Uuid, Path, description = "用户 UUID"),
    ),
    responses(
        (status = 200, description = "删除成功", body = ApiResponse<serde_json::Value>),
    ),
)]
pub async fn delete_user(
    State(db): State<DatabaseConnection>,
    Extension(current_user): Extension<User>,
    Path(user_id): Path<Uuid>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    // 安全约束：不能删除自己
    if current_user.id == user_id {
        return Err(AppError::bad_request("不能删除当前登录用户"));
    }

    // 删除用户
    UserAdminService::delete_user(&db, user_id)
        .await
        .map_err(|e| AppError::internal_error(e.to_string()))?
        .ok_or_else(|| AppError::not_found("用户不存在"))?;

    Ok(Json(ApiResponse::success(())))
}
