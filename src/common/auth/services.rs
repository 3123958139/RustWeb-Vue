//! # 认证服务模块
//!
//! 提供认证相关的业务逻辑：
//!
//! | 服务 | 功能 | 调用方 |
//! |------|------|--------|
//! | `login` | 验证邮箱和密码 | 登录处理器 |
//! | `get_user_by_id` | 根据 ID 获取用户 | 中间件 |
//! | `create_user` | 创建新用户 | 管理员处理器 |
//! | `create_default_settings` | 创建默认设置 | `create_user` |
//!
//! # 安全说明
//!
//! - 密码使用 bcrypt 加密（不可逆）
//! - 公开注册已移除，用户只能由管理员创建
//! - 创建用户时验证角色是否在注册表中

use crate::common::models::{LoginRequest, User, UserSettings};  // 数据模型
use crate::database::DatabaseConnection;                        // 数据库连接池
use bcrypt::{hash, verify, DEFAULT_COST};                      // 密码加密
use chrono::Utc;                                                // UTC 时间
use uuid::Uuid;                                                 // UUID 生成

/// 认证服务结构体
///
/// # 说明
///
/// 使用结构体组织相关函数（类似 Java 的静态方法类）。
/// 不包含状态，仅作为函数的命名空间。
pub struct AuthService;

impl AuthService {
    /// 用户登录验证
    ///
    /// # 参数
    /// - `pool`: 数据库连接池
    /// - `login_data`: 登录请求数据（邮箱 + 密码）
    ///
    /// # 返回值
    /// - `Ok(User)` - 验证成功，返回用户信息
    /// - `Err(...)` - 邮箱不存在或密码错误
    ///
    /// # 实现流程
    ///
    /// 1. 计算邮箱指纹（email_hash）定位用户
    ///    （邮箱在库中为 AES-256-GCM 密文，无法直接等值查找）
    /// 2. 验证密码（bcrypt 比较，在阻塞线程池中执行，避免占用 async worker）
    /// 3. 返回用户信息
    ///
    /// # 语法说明
    ///
    /// - `sqlx::query_as::<_, User>(SQL)`: 执行 SQL 并映射到 `User` 结构体
    ///   - 第一个泛型 `_` 让编译器推断
    ///   - 第二个泛型 `User` 指定目标类型
    /// - `.bind(value)`: 绑定参数（防止 SQL 注入）
    /// - `.fetch_optional(pool)`: 查询单条记录，不存在返回 `None`
    /// - `?`: 传播错误
    pub async fn login(
        pool: &DatabaseConnection,
        login_data: LoginRequest,
    ) -> Result<User, Box<dyn std::error::Error>> {
        // 按邮箱指纹查找用户（邮箱以密文存储，指纹为确定性查找键）
        // `fetch_optional` 返回 `Option<User>`：找到返回 `Some`，未找到返回 `None`
        // 登录需要密码哈希做校验，显式列出全部列
        let email_hash = crate::common::crypto::field_hash(&login_data.email);
        let user = sqlx::query_as::<_, User>(
            "SELECT id, username, email, password_hash, role, created_at, updated_at FROM users WHERE email_hash = $1",
        )
            .bind(&email_hash)
            .fetch_optional(pool)
            .await?
            .ok_or("用户不存在")?;  // `None` 时返回错误

        // 验证密码
        // `bcrypt::verify()` 比较明文密码和哈希值（CPU 密集操作）
        // `spawn_blocking` 将其移到阻塞线程池执行，避免阻塞 tokio worker
        let password = login_data.password;
        let expected_hash = user.password_hash.clone();
        let is_valid = tokio::task::spawn_blocking(move || {
            verify(password.as_bytes(), &expected_hash)
        })
        .await??;
        if !is_valid {
            return Err("密码错误".into());
        }

        Ok(user)
    }

    /// 根据用户 ID 获取用户信息
    ///
    /// # 参数
    /// - `pool`: 数据库连接池
    /// - `user_id`: 用户 UUID
    ///
    /// # 返回值
    /// - `Ok(Some(User))` - 找到用户
    /// - `Ok(None)` - 用户不存在
    /// - `Err(...)` - 数据库错误
    pub async fn get_user_by_id(
        pool: &DatabaseConnection,
        user_id: Uuid,
    ) -> Result<Option<User>, Box<dyn std::error::Error>> {
        // 中间件每请求调用：只查必要列，不取 password_hash（见 User 的 #[sqlx(default)]）
        let user =
            sqlx::query_as::<_, User>("SELECT id, username, email, role, created_at, updated_at FROM users WHERE id = $1")
                .bind(user_id)
                .fetch_optional(pool)
                .await?;

        Ok(user)
    }

    /// 创建新用户
    ///
    /// # 参数
    /// - `pool`: 数据库连接池
    /// - `username`: 用户名
    /// - `email`: 邮箱
    /// - `password`: 明文密码
    /// - `role`: 角色标识
    ///
    /// # 返回值
    /// - `Ok(User)` - 创建成功
    /// - `Err(...)` - 用户名/邮箱已存在或其他错误
    ///
    /// # 实现流程
    ///
    /// 1. 检查用户名（按指纹）/邮箱是否已存在
    /// 2. 加密密码（bcrypt）
    /// 3. 加密用户名（AES-256-GCM）并派生指纹
    /// 4. 插入数据库
    /// 5. 创建默认设置
    /// 6. 返回用户信息
    ///
    /// # 语法说明
    ///
    /// - `Uuid::new_v4()`: 生成随机 UUID（版本 4）
    /// - `RETURNING *`: SQLite 3.35+ 支持，返回插入的行
    /// - `.fetch_one(pool)`: 查询单条记录
    pub async fn create_user(
        pool: &DatabaseConnection,
        username: &str,
        email: &str,
        password: &str,
        role: &str,
    ) -> Result<User, Box<dyn std::error::Error>> {
        // 用户名/邮箱指纹：AES-256-GCM 密文无法直接比较明文查重，
        // 用确定性的 HMAC 指纹兜底唯一性（username_hash / email_hash）
        let username_hash = crate::common::crypto::username_hash(username);
        let email_hash = crate::common::crypto::field_hash(email);

        // 检查用户是否已存在（只查存在性，避免取整行）
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM users WHERE email_hash = $1 OR username_hash = $2)",
        )
        .bind(&email_hash)
        .bind(&username_hash)
        .fetch_one(pool)
        .await?;

        if exists {
            return Err("用户名或邮箱已存在".into());
        }

        // 加密密码
        // `hash()` 将明文密码转换为 bcrypt 哈希（CPU 密集操作）
        // `spawn_blocking` 将其移到阻塞线程池执行，避免阻塞 tokio worker
        // `DEFAULT_COST` 是工作因子（默认 10），越大越安全但越慢
        let password_owned = password.to_string();
        let password_hash = tokio::task::spawn_blocking(move || {
            hash(password_owned.as_bytes(), DEFAULT_COST)
        })
        .await??;

        // 加密用户名与邮箱（防止直接读取 SQLite 文件看到明文）
        let username_enc = crate::common::crypto::encrypt(username)?;
        let email_enc = crate::common::crypto::encrypt(email)?;
        // 角色同样加密入库（权限判断在读出行时已解密）
        let role_enc = crate::common::crypto::encrypt(role)?;

        // 创建用户
        let user_id = Uuid::new_v4();
        let now = Utc::now();
        let user = sqlx::query_as::<_, User>(
            r#"
            INSERT INTO users (id, username, username_hash, email, email_hash, password_hash, role, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING *
            "#,
        )
        .bind(user_id)
        .bind(&username_enc)
        .bind(&username_hash)
        .bind(&email_enc)
        .bind(&email_hash)
        .bind(&password_hash)
        .bind(&role_enc)
        .bind(now)
        .bind(now)
        .fetch_one(pool)
        .await?;

        // 创建默认设置
        Self::create_default_settings(pool, user.id).await?;

        Ok(user)
    }

    /// 创建用户默认设置
    ///
    /// # 参数
    /// - `pool`: 数据库连接池
    /// - `user_id`: 用户 UUID
    ///
    /// # 返回值
    /// - `Ok(UserSettings)` - 创建成功
    /// - `Err(...)` - 数据库错误
    ///
    /// # 默认设置
    ///
    /// | 设置 | 默认值 |
    /// |------|--------|
    /// | 主题 | light |
    /// | 主色调 | #409eff |
    /// | 邮件通知 | true |
    /// | 浏览器通知 | false |
    /// | 通知类型 | ["system"] |
    /// | 双因素认证 | false |
    /// | 会话超时 | 60 分钟 |
    /// | 语言 | zh-CN |
    /// | 时区 | Asia/Shanghai |
    pub async fn create_default_settings(
        pool: &DatabaseConnection,
        user_id: Uuid,
    ) -> Result<UserSettings, Box<dyn std::error::Error>> {
        let settings = sqlx::query_as::<_, UserSettings>(
            r#"
            INSERT INTO user_settings (
                id, user_id, theme, primary_color, email_notifications,
                browser_notifications, notification_types, two_factor_auth,
                session_timeout, profile_visibility, default_post_visibility,
                data_collection, language, timezone, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
            RETURNING *
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(user_id)
        .bind("light")
        .bind("#409eff")
        .bind(true)
        .bind(false)
        .bind(serde_json::to_string(&vec!["system"])?)
        .bind(false)
        .bind(60)
        .bind("public")
        .bind("public")
        .bind(true)
        .bind("zh-CN")
        .bind("Asia/Shanghai")
        .bind(Utc::now())
        .bind(Utc::now())
        .fetch_one(pool)
        .await?;

        Ok(settings)
    }
}
