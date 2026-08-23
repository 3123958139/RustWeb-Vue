//! # JWT 令牌模块
//!
//! 本模块提供 JWT（JSON Web Token）令牌的创建和验证功能。
//!
//! # JWT 结构
//!
//! JWT 由三部分组成，用 `.` 分隔：
//! ```text
//! Header.Payload.Signature
//! ```
//!
//! - **Header**: 算法和令牌类型
//! - **Payload**: 载荷数据（用户ID、过期时间等）
//! - **Signature**: 签名（防止篡改）
//!
//! # 工作流程
//!
//! ```text
//! 登录 → 验证密码 → 创建 JWT（含用户ID和过期时间）
//!       → 返回令牌给前端
//!
//! 后续请求 → 前端在 Authorization 头携带令牌
//!          → 后端验证令牌 → 提取用户ID → 查找用户
//! ```
//!
//! # 安全配置
//!
//! | 环境变量 | 默认值 | 说明 |
//! |----------|--------|------|
//! | `JWT_SECRET` | dev: "dev-insecure-secret-key" | 签名密钥（生产模式必须设置，否则拒绝启动） |
//! | `JWT_EXPIRATION` | "86400" | 过期时间（秒，默认 24 小时） |
//!
//! 密钥与过期时间在 `init()` 时读取一次并缓存到 `OnceLock`，
//! 避免每个请求重复读取环境变量（原实现还有兜底硬编码密钥的漏洞：
//! 部署忘记设置 `JWT_SECRET` 时任意攻击者可用默认密钥伪造 token）。

use crate::common::models::User;  // 用户结构体
// `jsonwebtoken` 库提供 JWT 编码/解码功能（>=10.3.0，修复 CVE-2026-25537）
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};  // 序列化/反序列化
use std::sync::OnceLock;              // 一次性初始化容器
use uuid::Uuid;                       // UUID 类型

/// JWT 载荷（Claims）结构体
///
/// 包含令牌的元数据，编码到 JWT 的 Payload 部分。
///
/// # 字段说明
///
/// | 字段 | 类型 | 说明 |
/// |------|------|------|
/// | `sub` | `String` | 主题（用户 ID） |
/// | `exp` | `u64` | 过期时间（Unix 时间戳，秒） |
/// | `iat` | `u64` | 签发时间（Unix 时间戳，秒） |
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    /// 主题（Subject）：存储用户 ID（UUID 字符串格式）
    pub sub: String,
    /// 过期时间（Expiration Time）：Unix 时间戳（秒）
    pub exp: u64,
    /// 签发时间（Issued At）：Unix 时间戳（秒）
    pub iat: u64,
}

/// 签名密钥（启动时初始化一次）
static JWT_SECRET: OnceLock<String> = OnceLock::new();
/// 令牌过期时间（秒，启动时初始化一次）
static JWT_EXPIRATION_SECS: OnceLock<u64> = OnceLock::new();

/// 初始化 JWT 配置（必须在创建/验证令牌前调用，`main.rs` 启动时执行）
///
/// # 安全说明
///
/// - 生产模式（`!cfg!(debug_assertions)`）下 `JWT_SECRET` 缺失时**拒绝启动**，
///   避免使用可被伪造的默认密钥
/// - dev 模式允许缺省（使用固定 dev 密钥），便于本地开发
pub fn init() -> Result<(), String> {
    let dev = cfg!(debug_assertions);
    let secret = match std::env::var("JWT_SECRET") {
        Ok(s) if !s.trim().is_empty() => s,
        _ if dev => {
            tracing::warn!("JWT_SECRET 未设置，开发模式使用默认密钥（生产模式将拒绝启动）");
            "dev-insecure-secret-key".to_string()
        }
        _ => {
            return Err(
                "JWT_SECRET 环境变量未设置（生产模式必须配置，否则令牌可被伪造）。\
                 请在 .env 或系统环境变量中设置一个随机长密钥"
                    .into(),
            )
        }
    };
    let expiration = std::env::var("JWT_EXPIRATION")
        .ok()
        .and_then(|v| v.trim().parse::<u64>().ok())
        .unwrap_or(86400);

    let _ = JWT_SECRET.set(secret);
    let _ = JWT_EXPIRATION_SECS.set(expiration);
    Ok(())
}

/// 获取缓存后的签名密钥
///
/// `pub(crate)`：供 `common::crypto` 派生用户名/邮箱/角色等的加解密与指纹密钥，
/// 保证字段密钥与 JWT 密钥同源单一。
pub(crate) fn secret() -> &'static str {
    JWT_SECRET.get().expect("jwt::init() 必须在创建/验证令牌前调用")
}

/// 获取缓存后的过期时间（秒）
fn expiration_secs() -> u64 {
    *JWT_EXPIRATION_SECS.get().unwrap_or(&86400)
}

/// 创建 JWT 令牌
///
/// # 参数
/// - `user`: 用户信息（用于提取用户 ID）
///
/// # 返回值
/// - `Ok(String)` - 成功返回 JWT 令牌字符串
/// - `Err(Error)` - 签名失败
///
/// # 实现细节
///
/// 1. 使用启动时缓存的密钥与过期时间（`OnceLock`，零重复环境变量读取）
/// 2. 构建 Claims（用户 ID + 过期时间 + 签发时间）
/// 3. 使用 HS256 算法签名
pub fn create_token(user: &User) -> Result<String, jsonwebtoken::errors::Error> {
    let now = chrono::Utc::now();
    let claims = Claims {
        sub: user.id.to_string(),                       // 将 UUID 转换为字符串
        exp: (now.timestamp() + expiration_secs() as i64) as u64,  // 过期时间戳
        iat: now.timestamp() as u64,                    // 签发时间戳
    };

    encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(secret().as_bytes()),
    )
}

/// 验证 JWT 令牌并提取用户 ID
///
/// # 参数
/// - `token`: JWT 令牌字符串
///
/// # 返回值
/// - `Ok(Uuid)` - 成功返回用户 ID
/// - `Err(Error)` - 令牌无效或已过期
///
/// # 验证内容
///
/// 1. 令牌格式是否正确（三部分用 `.` 分隔）
/// 2. 签名是否有效（防篡改）
/// 3. 是否已过期（`exp` 校验，jsonwebtoken 10.3+ 修复了畸形类型绕过问题）
pub fn verify_token(token: &str) -> Result<Uuid, jsonwebtoken::errors::Error> {
    let validation = Validation::new(Algorithm::HS256);

    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret().as_bytes()),
        &validation,
    )?;

    Uuid::parse_str(&token_data.claims.sub)
        .map_err(|_| jsonwebtoken::errors::Error::from(jsonwebtoken::errors::ErrorKind::InvalidToken))
}
