use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use validator::Validate;

// 用户角色枚举
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum UserRole {
    Admin,
    Moderator,
    User,
}

impl UserRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            UserRole::Admin => "admin",
            UserRole::Moderator => "moderator",
            UserRole::User => "user",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "admin" => UserRole::Admin,
            "moderator" => UserRole::Moderator,
            "user" => UserRole::User,
            _ => UserRole::User,
        }
    }

    pub fn permissions(&self) -> Vec<Permission> {
        match self {
            UserRole::Admin => vec![
                Permission::Dashboard,
                Permission::PostsRead,
                Permission::PostsWrite,
                Permission::PostsDelete,
                Permission::UsersRead,
                Permission::UsersWrite,
                Permission::UsersDelete,
                Permission::Settings,
                Permission::SystemAdmin,
            ],
            UserRole::Moderator => vec![
                Permission::Dashboard,
                Permission::PostsRead,
                Permission::PostsWrite,
                Permission::PostsDelete,
                Permission::UsersRead,
                Permission::Settings,
            ],
            UserRole::User => vec![
                Permission::Dashboard,
                Permission::PostsRead,
                Permission::PostsWrite,
                Permission::Settings,
            ],
        }
    }
}

// 权限枚举
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub enum Permission {
    Dashboard,
    PostsRead,
    PostsWrite,
    PostsDelete,
    UsersRead,
    UsersWrite,
    UsersDelete,
    Settings,
    SystemAdmin,
}

impl Permission {
    pub fn as_str(&self) -> &'static str {
        match self {
            Permission::Dashboard => "dashboard",
            Permission::PostsRead => "posts:read",
            Permission::PostsWrite => "posts:write",
            Permission::PostsDelete => "posts:delete",
            Permission::UsersRead => "users:read",
            Permission::UsersWrite => "users:write",
            Permission::UsersDelete => "users:delete",
            Permission::Settings => "settings",
            Permission::SystemAdmin => "system:admin",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "dashboard" => Some(Permission::Dashboard),
            "posts:read" => Some(Permission::PostsRead),
            "posts:write" => Some(Permission::PostsWrite),
            "posts:delete" => Some(Permission::PostsDelete),
            "users:read" => Some(Permission::UsersRead),
            "users:write" => Some(Permission::UsersWrite),
            "users:delete" => Some(Permission::UsersDelete),
            "settings" => Some(Permission::Settings),
            "system:admin" => Some(Permission::SystemAdmin),
            _ => None,
        }
    }
}

// 菜单项结构体（用于前端显示）
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MenuItemDisplay {
    pub id: String,
    pub title: String,
    pub path: String,
    pub icon: String,
    pub permissions: Vec<Permission>,
    pub children: Option<Vec<MenuItemDisplay>>,
}

// 用户权限结构体
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserPermissions {
    pub user_id: Uuid,
    pub role: UserRole,
    pub permissions: Vec<Permission>,
}

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub role: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl User {
    pub fn get_role(&self) -> UserRole {
        UserRole::from_str(&self.role)
    }

    pub fn has_permission(&self, permission: &Permission) -> bool {
        self.get_role().permissions().contains(permission)
    }

    pub fn get_permissions(&self) -> Vec<Permission> {
        self.get_role().permissions()
    }
}

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct Post {
    pub id: Uuid,
    pub title: String,
    pub content: String,
    pub author_id: Uuid,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateUserRequest {
    #[validate(length(min = 3, max = 50))]
    pub username: String,
    #[validate(email)]
    pub email: String,
    #[validate(length(min = 6))]
    pub password: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct LoginRequest {
    #[validate(email)]
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub user: User,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreatePostRequest {
    #[validate(length(min = 1, max = 200))]
    pub title: String,
    #[validate(length(min = 1))]
    pub content: String,
    pub status: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub message: Option<String>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            message: None,
        }
    }

    pub fn error(message: String) -> Self {
        Self {
            success: false,
            data: None,
            message: Some(message),
        }
    }
}

// 用户设置结构体
#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct UserSettings {
    pub id: Uuid,
    pub user_id: Uuid,
    pub theme: String,
    pub primary_color: String,
    pub email_notifications: bool,
    pub browser_notifications: bool,
    pub notification_types: Vec<String>,
    pub two_factor_auth: bool,
    pub session_timeout: i32,
    pub profile_visibility: String,
    pub default_post_visibility: String,
    pub data_collection: bool,
    pub language: String,
    pub timezone: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// 菜单管理相关结构体
#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct MenuItem {
    pub id: Uuid,
    pub title: String,
    pub path: Option<String>,
    pub icon: Option<String>,
    pub parent_id: Option<Uuid>,
    pub sort_order: i32,
    pub enabled: bool,
    pub permissions: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct CreateMenuItemRequest {
    #[validate(length(min = 1, max = 100))]
    pub title: String,
    #[validate(length(max = 200))]
    pub path: Option<String>,
    #[validate(length(max = 50))]
    pub icon: Option<String>,
    pub parent_id: Option<Uuid>,
    pub sort_order: i32,
    pub enabled: bool,
    pub permissions: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct UpdateMenuItemRequest {
    #[validate(length(min = 1, max = 100))]
    pub title: Option<String>,
    #[validate(length(max = 200))]
    pub path: Option<String>,
    #[validate(length(max = 50))]
    pub icon: Option<String>,
    pub parent_id: Option<Uuid>,
    pub sort_order: Option<i32>,
    pub enabled: Option<bool>,
    pub permissions: Option<Vec<String>>,
}

// 角色管理相关结构体
#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct Role {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub permissions: Vec<String>,
    pub user_count: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct CreateRoleRequest {
    #[validate(length(min = 1, max = 50))]
    pub name: String,
    #[validate(length(min = 1, max = 500))]
    pub description: String,
    pub permissions: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct UpdateRoleRequest {
    #[validate(length(min = 1, max = 50))]
    pub name: Option<String>,
    #[validate(length(min = 1, max = 500))]
    pub description: Option<String>,
    pub permissions: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct UpdateUserRoleRequest {
    pub role: String,
}

// 创建设置请求
#[derive(Debug, Deserialize, Validate)]
pub struct CreateSettingsRequest {
    pub theme: Option<String>,
    pub primary_color: Option<String>,
    pub email_notifications: Option<bool>,
    pub browser_notifications: Option<bool>,
    pub notification_types: Option<Vec<String>>,
    pub two_factor_auth: Option<bool>,
    pub session_timeout: Option<i32>,
    pub profile_visibility: Option<String>,
    pub default_post_visibility: Option<String>,
    pub data_collection: Option<bool>,
    pub language: Option<String>,
    pub timezone: Option<String>,
}

// 更新设置请求
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateSettingsRequest {
    pub theme: Option<String>,
    pub primary_color: Option<String>,
    pub email_notifications: Option<bool>,
    pub browser_notifications: Option<bool>,
    pub notification_types: Option<Vec<String>>,
    pub two_factor_auth: Option<bool>,
    pub session_timeout: Option<i32>,
    pub profile_visibility: Option<String>,
    pub default_post_visibility: Option<String>,
    pub data_collection: Option<bool>,
    pub language: Option<String>,
    pub timezone: Option<String>,
}

// 设备信息结构体
#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct UserDevice {
    pub id: Uuid,
    pub user_id: Uuid,
    pub device_name: String,
    pub browser: String,
    pub location: String,
    pub ip_address: String,
    pub user_agent: String,
    pub last_login: DateTime<Utc>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

// 导出数据请求
#[derive(Debug, Deserialize, Validate)]
pub struct ExportDataRequest {
    #[validate(email)]
    pub email: String,
    pub data_types: Vec<String>,
}

// 删除账户请求
#[derive(Debug, Deserialize, Validate)]
pub struct DeleteAccountRequest {
    pub password: String,
    pub confirmation: String,
}
