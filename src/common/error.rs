//! # 统一错误处理模块
//!
//! 本模块定义应用程序的统一错误类型 `AppError`，并实现与 Axum 的集成。
//!
//! # 设计理念
//!
//! 将所有错误类型统一为 `AppError`，实现：
//! - **类型安全**: 编译时检查错误处理
//! - **统一格式**: 所有错误返回相同的 JSON 格式
//! - **自动转换**: 通过 `From` trait 自动转换各种错误类型
//!
//! # 错误响应格式
//!
//! ```json
//! {
//!     "success": false,
//!     "message": "错误描述信息"
//! }
//! ```
//!
//! # 错误类型映射
//!
//! | 错误类型 | HTTP 状态码 | 说明 |
//! |----------|-------------|------|
//! | 数据验证失败 | 400 Bad Request | 请求参数无效 |
//! | 未认证 | 401 Unauthorized | 令牌无效或缺失 |
//! | 无权限 | 403 Forbidden | 用户无权访问 |
//! | 资源不存在 | 404 Not Found | 记录未找到 |
//! | 数据已存在 | 400 Bad Request | 唯一键冲突（如重复邮箱） |
//! | 密码处理错误 | 500 Internal Server Error | bcrypt 加密失败 |
//! | Token 无效 | 401 Unauthorized | JWT 验证失败 |
//! | 数据库错误 | 500 Internal Server Error | SQL 执行失败 |

use axum::{
    http::StatusCode,                    // HTTP 状态码
    response::{IntoResponse, Response}, // 响应转换 trait
    Json,                               // JSON 响应
};
use serde_json::json;  // 快速构建 JSON 值
use std::fmt;          // 格式化 trait

/// 应用统一错误类型
///
/// # 说明
///
/// 所有内部错误都转换为 `AppError`，然后通过 `IntoResponse` 转换为 HTTP 响应。
///
/// # 语法说明
///
/// `#[derive(Debug)]` 自动实现 `Debug` trait，支持 `{:?}` 格式化输出，
/// 这在日志记录和错误调试时很有用。
#[derive(Debug)]
pub struct AppError {
    /// HTTP 状态码（如 400、401、404、500）
    pub status_code: StatusCode,
    /// 错误描述信息（返回给前端显示）
    pub message: String,
}

impl AppError {
    /// 创建自定义状态码的错误
    ///
    /// # 参数
    /// - `status_code`: HTTP 状态码
    /// - `message`: 错误描述
    pub fn new(status_code: StatusCode, message: impl Into<String>) -> Self {
        Self {
            status_code,
            message: message.into(),  // `impl Into<String>` 允许传入 `&str` 或 `String`
        }
    }

    /// 创建 400 Bad Request 错误
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, message)
    }

    /// 创建 401 Unauthorized 错误
    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self::new(StatusCode::UNAUTHORIZED, message)
    }

    /// 创建 403 Forbidden 错误
    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::new(StatusCode::FORBIDDEN, message)
    }

    /// 创建 404 Not Found 错误
    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_FOUND, message)
    }

    /// 创建 500 Internal Server Error 错误
    pub fn internal_error(message: impl Into<String>) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, message)
    }

    /// 创建 429 Too Many Requests 错误（速率限制）
    pub fn too_many_requests(message: impl Into<String>) -> Self {
        Self::new(StatusCode::TOO_MANY_REQUESTS, message)
    }
}

// ============ 标准库 trait 实现 ============

/// 实现 `Display` trait，支持 `format!("{}", error)` 格式化
impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

/// 实现 `Error` trait，使 `AppError` 成为标准错误类型
impl std::error::Error for AppError {}

/// 实现 `IntoResponse` trait，使 `AppError` 可以作为 Axum 响应
///
/// # 说明
///
/// 当处理器返回 `Result<_, AppError>` 时，Axum 会自动调用此方法
/// 将错误转换为 HTTP 响应。
///
/// # 响应格式
///
/// ```json
/// {
///     "success": false,
///     "message": "错误描述"
/// }
/// ```
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        // 构建 JSON 响应体
        let body = Json(json!({
            "success": false,
            "message": self.message
        }));

        // 返回状态码和 JSON 响应
        (self.status_code, body).into_response()
    }
}

// ============ 错误自动转换（From trait） ============

/// 从 `sqlx::Error` 自动转换为 `AppError`
///
/// # 说明
///
/// 使用 `?` 操作符时，Rust 会自动调用 `From` trait 进行转换。
/// 例如：`sqlx::query(...).execute(pool).await?` 如果出错，
/// 会自动转换为 `AppError`。
///
/// # 错误映射
///
/// | SQLx 错误 | AppError | 说明 |
/// |-----------|----------|------|
/// | `RowNotFound` | 404 Not Found | 记录未找到 |
/// | `Database` (23505) | 400 Bad Request | 唯一键冲突 |
/// | 其他 `Database` | 500 Internal Server Error | 其他数据库错误 |
/// | 其他 | 500 Internal Server Error | 未知数据库错误 |
impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => AppError::not_found("记录未找到"),
            sqlx::Error::Database(db_err) => {
                // SQLite 错误代码 23505 = UNIQUE 约束违反
                if db_err.code() == Some(std::borrow::Cow::Borrowed("23505")) {
                    AppError::bad_request("数据已存在")
                } else {
                    AppError::internal_error("数据库错误")
                }
            }
            _ => AppError::internal_error("数据库错误"),
        }
    }
}

/// 从 `bcrypt::BcryptError` 自动转换为 `AppError`
impl From<bcrypt::BcryptError> for AppError {
    fn from(_: bcrypt::BcryptError) -> Self {
        AppError::internal_error("密码处理错误")
    }
}

/// 从 `jsonwebtoken::errors::Error` 自动转换为 `AppError`
impl From<jsonwebtoken::errors::Error> for AppError {
    fn from(_: jsonwebtoken::errors::Error) -> Self {
        AppError::unauthorized("Token无效")
    }
}

/// 从 `validator::ValidationErrors` 自动转换为 `AppError`
///
/// # 说明
///
/// 将验证错误格式化为用户友好的消息：
/// ```text
/// email: 请输入有效的邮箱地址; username: 长度必须在3到50之间
/// ```
impl From<validator::ValidationErrors> for AppError {
    fn from(errors: validator::ValidationErrors) -> Self {
        let message = errors
            .field_errors()
            .iter()
            .map(|(field, errors)| {
                format!(
                    "{}: {}",
                    field,
                    errors
                        .iter()
                        .map(|e| e.to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            })
            .collect::<Vec<_>>()
            .join("; ");

        AppError::bad_request(message)
    }
}
