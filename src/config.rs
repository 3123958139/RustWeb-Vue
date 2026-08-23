//! # 应用配置模块
//!
//! 本模块负责从环境变量加载应用配置。
//!
//! # 配置项
//!
//! | 环境变量 | 类型 | 默认值 | 说明 |
//! |----------|------|--------|------|
//! | `PORT` | u16 | 3000 | 服务器监听端口 |
//! | `DATABASE_URL` | String | `sqlite://fj200c.db` | 数据库连接 URL |
//!
//! # 使用方式
//!
//! ```bash
//! # 设置环境变量
//! export PORT=8080
//! export DATABASE_URL=sqlite://mydb.db
//!
//! # 或在 .env 文件中配置
//! PORT=8080
//! DATABASE_URL=sqlite://mydb.db
//! ```
//!
//! # 配置加载流程
//!
//! 1. `dotenv::dotenv()` 加载 `.env` 文件
//! 2. `env::var()` 读取环境变量
//! 3. `unwrap_or_else()` 提供默认值
//! 4. `parse()` 解析为具体类型

use serde::Deserialize;  // 反序列化 trait，用于从环境变量字符串解析配置
use std::env;            // 标准库环境变量模块

/// 应用配置结构体
///
/// # 设计理念
///
/// - 使用 `#[derive(Deserialize)]` 自动实现反序列化
/// - 所有字段为 `pub`，方便外部访问
/// - 使用 `#[derive(Debug)]` 支持调试输出
#[derive(Debug, Deserialize)]
pub struct AppConfig {
    /// 服务器监听端口（默认 3000）
    /// 端口范围：0-65535，其中 0-1023 是特权端口（需要管理员权限）
    pub port: u16,
    
    /// SQLite 数据库连接 URL
    /// 格式：`sqlite://<文件路径>`
    /// 例如：`sqlite://fj200c.db`（当前目录的 fj200c.db 文件）
    pub database_url: String,
}

impl AppConfig {
    /// 从环境变量加载配置
    ///
    /// # 返回值
    /// - `Ok(AppConfig)` - 成功返回配置
    /// - `Err(Box<dyn Error>)` - 失败返回错误（如端口号不是有效数字）
    ///
    /// # 错误处理
    ///
    /// - `PORT` 不是有效数字 → 返回 `ParseIntError`
    /// - 其他环境变量缺失 → 使用默认值
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        // 读取 PORT 环境变量
        // `env::var()` 返回 `Result<String, VarError>`
        // `unwrap_or_else()` 在错误时提供默认值 "3000"
        // `.parse::<u16>()` 将字符串解析为 u16 整数
        // `?` 操作符传播解析错误
        let port = env::var("PORT")
            .unwrap_or_else(|_| "3000".to_string())
            .parse::<u16>()?;

        // 读取 DATABASE_URL 环境变量
        // 默认值为 `sqlite://fj200c.db`（当前目录）
        let database_url = env::var("DATABASE_URL")
            .unwrap_or_else(|_| "sqlite://fj200c.db".to_string());

        Ok(AppConfig {
            port,
            database_url,
        })
    }
}
