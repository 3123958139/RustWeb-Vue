//! # 数据模型模块
//!
//! 本模块定义系统中使用的所有数据结构：
//!
//! - `Permission`: 权限枚举
//! - `User`: 用户结构体
//! - `LoginRequest/Response`: 登录请求/响应
//! - `CreateUserRequest`: 创建用户请求
//! - `ApiResponse`: 统一 API 响应格式
//! - `UserSettings`: 用户设置
//!
//! # 架构说明
//!
//! 数据模型是系统的**核心抽象层**：
//! - **前端 ↔ 后端**: 通过 JSON 序列化/反序列化
//! - **后端 ↔ 数据库**: 通过 SQLx 的 `FromRow` 自动映射
//!
//! # 设计理念
//!
//! - **类型安全**: 使用 Rust 类型系统防止错误
//! - **序列化支持**: `Serialize` 用于输出，`Deserialize` 用于输入
//! - **验证规则**: `#[validate(...)]` 注解自动验证输入
//! - **数据库映射**: `FromRow` 自动从数据库行映射到结构体

// ============ 导入依赖 ============
use chrono::{DateTime, Utc};
// 日期时间类型（UTC 时区）
use serde::{Deserialize, Serialize};
// 序列化/反序列化
use sqlx::sqlite::SqliteRow;
// SQLite 行类型
use sqlx::{FromRow, Row};
// SQLx 数据库行映射
use uuid::Uuid;
// UUID 类型
use validator::Validate;
// 输入验证

use crate::roles;
// 引入角色注册表（用于解析用户权限）

// ============ 权限枚举 ============
/// 权限枚举
///
/// 定义系统中所有可能的权限点。
/// 角色注册表（`roles.rs`）引用这些权限点。
///
/// # RBAC 权限模型
///
/// ```text
/// 用户 → 角色 → 权限
/// ```
///
/// 例如：
/// - 用户 A 的角色是 `admin`
/// - `admin` 角色拥有 `[UsersRead, UsersWrite, UsersDelete, SystemAdmin]`
/// - 用户 A 拥有这些权限
///
/// # 语法说明
///
/// `#[derive(...)]` 是 Rust 的过程宏（proc macro），自动为枚举实现：
/// - `Debug`: 支持 `{:?}` 格式化输出
/// - `Serialize`: 支持序列化为 JSON
/// - `Deserialize`: 支持从 JSON 反序列化
/// - `Clone`: 支持克隆
/// - `PartialEq`: 支持 `==` 比较
/// - `Eq`: 支持完全相等比较
/// - `Hash`: 支持作为 HashMap 的键
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash, utoipa::ToSchema)]
pub enum Permission {
    // ============ 角色专属权限 ============
    /// fj200c_information 面板：发动机监控权限
    Fj200cInformationMonitor,
    /// fw150 面板：设备台账权限
    Fw100Monitor,
    /// ftj1c 面板：UDP 组播通信监控权限
    Ftj1cMonitor,
    /// city3d 面板：城市 3D 展示权限
    City3dView,
    /// fw150 面板：设备台账权限
    Fw150Monitor,
    /// fj200c_main 面板：发动机测控权限
    Fj200cMainMonitor,

    // ============ 管理面板权限 ============
    /// 用户读取权限（查看用户列表）
    UsersRead,
    /// 用户写入权限（创建/更新用户）
    UsersWrite,
    /// 用户删除权限
    UsersDelete,
    /// 系统管理标志（拥有此权限的角色视为管理角色）
    SystemAdmin,
}

// ============ 用户结构体 ============
/// 用户结构体，与数据库 `users` 表对应
///
/// # 字段说明
///
/// | 字段 | 类型 | 说明 |
/// |------|------|------|
/// | `id` | `Uuid` | 用户唯一标识 |
/// | `username` | `String` | 用户名 |
/// | `email` | `String` | 邮箱（用于登录） |
/// | `password_hash` | `String` | 密码哈希（不可序列化） |
/// | `role` | `String` | 角色标识 |
/// | `created_at` | `DateTime<Utc>` | 创建时间 |
/// | `updated_at` | `DateTime<Utc>` | 更新时间 |
///
/// # 语法说明
///
/// - `#[derive(FromRow)]`: SQLx 自动实现从数据库行映射到结构体
/// - `#[serde(skip_serializing)]`: 序列化时跳过此字段（密码哈希不应返回给前端）
#[derive(Debug, Serialize, Deserialize, FromRow, Clone, utoipa::ToSchema)]
pub struct User {
    /// 用户 UUID（主键）
    pub id: Uuid,
    /// 用户名（唯一）
    pub username: String,
    /// 邮箱（唯一，用于登录）
    pub email: String,
    /// 密码哈希（bcrypt 加密）
    /// `#[serde(skip_serializing)]` 确保 JSON 输出时不包含此字段
    /// `#[sqlx(default)]` 允许未查此列的查询（中间件/列表接口）映射为默认值
    #[serde(skip_serializing)]
    #[sqlx(default)]
    pub password_hash: String,
    /// 角色标识（如 "admin"、"fj200c_information"），与角色注册表的 `key` 对应
    pub role: String,
    /// 创建时间（UTC）
    pub created_at: DateTime<Utc>,
    /// 更新时间（UTC）
    pub updated_at: DateTime<Utc>,
}

impl User {
    /// 获取用户的权限列表
    ///
    /// 根据用户的 `role` 字段，查询角色注册表获取权限
    ///
    /// # 示例
    ///
    /// ```rust
    /// let permissions = user.permissions();
    /// if permissions.contains(&Permission::SystemAdmin) {
    ///     // 用户是管理员
    /// }
    /// ```
    pub fn permissions(&self) -> Vec<Permission> {
        roles::permissions_for(&self.role)
    }

    /// 检查用户是否拥有指定权限
    ///
    /// # 参数
    /// - `permission`: 要检查的权限
    pub fn has_permission(&self, permission: &Permission) -> bool {
        self.permissions().contains(permission)
    }
}

// ============ 登录请求 ============
/// 登录请求结构体，前端发送的登录数据
///
/// # 验证规则
/// - `email`: 必须是有效邮箱格式
/// - `password`: 无特殊要求（由服务端验证）
///
/// # 语法说明
///
/// `#[derive(Validate)]` 启用输入验证：
/// - `#[validate(email)]`: 验证邮箱格式
/// - 验证失败时返回 `validator::ValidationErrors`
#[derive(Debug, Deserialize, Validate, utoipa::ToSchema)]
pub struct LoginRequest {
    /// 邮箱（必填，必须是有效邮箱格式）
    #[validate(email)]
    pub email: String,
    /// 密码（必填）
    pub password: String,
}

// ============ 登录响应 ============
/// 登录响应结构体，登录成功后返回的数据
///
/// # 字段
/// - `token`: JWT 令牌（用于后续请求认证）
/// - `user`: 用户信息
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct LoginResponse {
    /// JWT 令牌
    /// 前端存储在 localStorage，每次请求放在 `Authorization` 头
    pub token: String,
    /// 用户信息
    pub user: User,
}

// ============ 创建用户请求 ============
/// 管理员创建用户请求
///
/// # 验证规则
/// - `username`: 长度 3-50 字符
/// - `email`: 必须是有效邮箱格式
/// - `password`: 长度至少 6 字符
/// - `role`: 可选，必须在角色注册表中
///
/// # 安全考虑
/// - 角色绑定由注册表白名单校验
/// - 防止创建拥有非法角色的用户
#[derive(Debug, Deserialize, Validate, utoipa::ToSchema)]
pub struct CreateUserRequest {
    /// 用户名（3-50 字符）
    #[validate(length(min = 3, max = 50))]
    pub username: String,
    /// 邮箱（必须是有效格式）
    #[validate(email)]
    pub email: String,
    /// 密码（至少 6 字符）
    #[validate(length(min = 6))]
    pub password: String,
    /// 角色标识（可选，不提供时默认为 "fj200c_information"）
    pub role: Option<String>,
}

// ============ 更新用户角色请求 ============
/// 管理员修改用户角色时发送的数据
#[derive(Debug, Deserialize, Validate, utoipa::ToSchema)]
pub struct UpdateUserRoleRequest {
    /// 新角色标识，必须在角色注册表中
    pub role: String,
}

// ============ 统一 API 响应 ============
/// 统一 API 响应格式
///
/// 所有 API 端点返回此格式，前端统一处理。
///
/// 成功响应：
/// ```json
/// { "success": true, "data": { ... }, "message": null }
/// ```
///
/// 错误响应：
/// ```json
/// { "success": false, "data": null, "message": "错误信息" }
/// ```
///
/// # 泛型说明
///
/// `T` 是泛型参数，表示 `data` 字段的类型：
/// - `ApiResponse<User>` - 单个用户
/// - `ApiResponse<Vec<User>>` - 用户列表
/// - `ApiResponse<LoginResponse>` - 登录响应
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ApiResponse<T> {
    /// 请求是否成功
    pub success: bool,
    /// 响应数据（可选）
    pub data: Option<T>,
    /// 错误信息（可选）
    pub message: Option<String>,
}

impl<T> ApiResponse<T> {
    /// 创建成功响应
    ///
    /// # 参数
    /// - `data`: 响应数据
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            message: None,
        }
    }

    /// 创建错误响应
    ///
    /// # 参数
    /// - `message`: 错误信息
    pub fn error(message: String) -> Self {
        Self {
            success: false,
            data: None,
            message: Some(message),
        }
    }
}

// ============ 用户设置结构体 ============
/// 用户设置结构体，与数据库 `user_settings` 表对应
///
/// # 字段说明
///
/// | 字段 | 类型 | 默认值 | 说明 |
/// |------|------|--------|------|
/// | `theme` | `String` | "light" | 主题（light/dark） |
/// | `primary_color` | `String` | "#409eff" | 主色调 |
/// | `email_notifications` | `bool` | true | 邮件通知 |
/// | `browser_notifications` | `bool` | false | 浏览器通知 |
/// | `notification_types` | `Vec<String>` | ["system"] | 通知类型 |
/// | `two_factor_auth` | `bool` | false | 双因素认证 |
/// | `session_timeout` | `i32` | 60 | 会话超时（分钟） |
/// | `profile_visibility` | `String` | "public" | 个人资料可见性 |
/// | `default_post_visibility` | `String` | "public" | 默认帖子可见性 |
/// | `data_collection` | `bool` | true | 数据收集 |
/// | `language` | `String` | "zh-CN" | 语言 |
/// | `timezone` | `String` | "Asia/Shanghai" | 时区 |
#[derive(Debug, Serialize, Deserialize, Clone, utoipa::ToSchema)]
pub struct UserSettings {
    /// 设置 UUID（主键）
    pub id: Uuid,
    /// 用户 UUID（外键，关联 users 表）
    pub user_id: Uuid,
    /// 主题（"light" 或 "dark"）
    pub theme: String,
    /// 主色调（十六进制颜色码）
    pub primary_color: String,
    /// 是否启用邮件通知
    pub email_notifications: bool,
    /// 是否启用浏览器通知
    pub browser_notifications: bool,
    /// 通知类型（JSON 数组，如 ["system", "alert"]）
    pub notification_types: Vec<String>,
    /// 是否启用双因素认证
    pub two_factor_auth: bool,
    /// 会话超时时间（分钟）
    pub session_timeout: i32,
    /// 个人资料可见性（"public" 或 "private"）
    pub profile_visibility: String,
    /// 默认帖子可见性
    pub default_post_visibility: String,
    /// 是否允许数据收集
    pub data_collection: bool,
    /// 语言（如 "zh-CN"、"en-US"）
    pub language: String,
    /// 时区（如 "Asia/Shanghai"）
    pub timezone: String,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 更新时间
    pub updated_at: DateTime<Utc>,
}

// ============ 手动实现 FromRow ============
/// 手动实现 `FromRow` trait
///
/// SQLx 的 `FromRow` trait 用于将数据库行映射到结构体。
/// 通常使用 `#[derive(FromRow)]` 自动实现，
/// 但 `notification_types` 字段需要手动解析 JSON 数组。
///
/// # 语法说明
///
/// `impl<'r> FromRow<'r, SqliteRow> for UserSettings` 实现了：
/// - `'r`: 生命周期参数，表示行数据的引用必须活到方法返回之后
/// - `FromRow<'r, SqliteRow>`: 从 SQLite 行映射到 UserSettings
impl<'r> FromRow<'r, SqliteRow> for UserSettings {
    fn from_row(row: &'r SqliteRow) -> Result<Self, sqlx::Error> {
        Ok(UserSettings {
            id: row.try_get("id")?,
            user_id: row.try_get("user_id")?,
            theme: row.try_get("theme")?,
            primary_color: row.try_get("primary_color")?,
            email_notifications: row.try_get("email_notifications")?,
            browser_notifications: row.try_get("browser_notifications")?,
            // `notification_types` 存储为 JSON 字符串，需要手动解析为 `Vec<String>`
            notification_types: parse_json_array(&row.try_get::<String, _>("notification_types")?),
            two_factor_auth: row.try_get("two_factor_auth")?,
            session_timeout: row.try_get("session_timeout")?,
            profile_visibility: row.try_get("profile_visibility")?,
            default_post_visibility: row.try_get("default_post_visibility")?,
            data_collection: row.try_get("data_collection")?,
            language: row.try_get("language")?,
            timezone: row.try_get("timezone")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }
}

/// 解析 JSON 数组字符串
///
/// # 参数
/// - `value`: JSON 数组字符串，如 `["system", "alert"]`
///
/// # 返回值
/// 解析后的字符串数组。解析失败时返回空数组。
fn parse_json_array(value: &str) -> Vec<String> {
    serde_json::from_str(value).unwrap_or_default()
}
