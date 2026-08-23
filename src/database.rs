//! # 数据库初始化模块
//!
//! 本模块负责 SQLite 数据库的初始化，包括：
//! 1. 创建数据库连接池
//! 2. 创建表结构（`users`、`user_settings`）
//! 3. 创建索引
//! 4. 幂等清理历史表（兼容旧版本）
//! 5. 插入种子数据（初始用户账号）
//!
//! # SQLite 特性
//!
//! - **轻量级**：无需安装独立服务器，单文件数据库
//! - **零配置**：无需创建用户、授权等
//! - **WAL 模式**：Write-Ahead Logging，支持并发读写
//! - **外键约束**：启用 `foreign_keys` 确保数据完整性
//!
//! # 种子数据
//!
//! 首次运行时自动创建以下账号，**密码为随机生成，不打印到日志**，
//! 初始密码明文仅写入 `seed_passwords` 表，通过 `GET /admin/pwd` 查询
//! （部署后请用查询到的密码登录并立即修改）：
//!
//! | 用户名 | 邮箱 | 角色 |
//! |--------|------|------|
//! | admin | admin@7304.com | admin |
//! | fj200c_information | fj200c_information@7304.com | fj200c_information |
//! | fj200c_main | fj200c_main@7304.com | fj200c_main |
//! | mario | mario@7304.com | mario |

use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions};
// SQLite 连接相关类型
use std::str::FromStr;
// 字符串解析 trait
use uuid::Uuid;
// UUID 生成库

/// 数据库连接池类型别名
/// `SqlitePool` 是 SQLx 提供的 SQLite 连接池，支持异步并发访问
pub type DatabaseConnection = SqlitePool;

/// 初始化数据库连接和表结构
///
/// # 参数
/// - `database_url`: SQLite 数据库文件路径，如 `sqlite://fj200c.db`
///
/// # 返回值
/// - `Ok(DatabaseConnection)` - 成功返回连接池
/// - `Err(...)` - 失败返回错误
///
/// # 执行流程
///
/// 1. 解析连接选项（创建文件、启用外键、WAL 模式）
/// 2. 创建连接池
/// 3. 调用 `create_tables()` 创建表结构
pub async fn init_database(
    database_url: &str,
) -> Result<DatabaseConnection, Box<dyn std::error::Error>> {
    println!("正在连接数据库: {}", database_url);

    // 配置 SQLite 连接选项
    // `SqliteConnectOptions` 提供细粒度的连接配置
    let options = SqliteConnectOptions::from_str(database_url)?
        .create_if_missing(true) // 数据库文件不存在时自动创建
        .foreign_keys(true) // 启用外键约束（默认关闭）
        .journal_mode(SqliteJournalMode::Wal) // 设置 WAL 日志模式（性能更好）
        .busy_timeout(std::time::Duration::from_secs(5)); // 写锁等待 5s，避免 SQLITE_BUSY

    // 创建连接池
    // 显式配置 `max_connections`，SQLite 同一时刻只有一个写事务，
    // 连接数过高只会加剧锁等待，10 个连接足够所有角色并发读
    let pool = SqlitePoolOptions::new()
        .max_connections(10)
        .connect_with(options)
        .await?;
    println!("数据库连接成功");

    // 创建表结构
    create_tables(&pool).await?;
    println!("数据库表创建完成");

    Ok(pool)
}

/// 创建数据库表结构和种子数据
///
/// # 表结构
///
/// ## users 表（用户表）
///
/// | 字段 | 类型 | 约束 | 说明 |
/// |------|------|------|------|
/// | id | BLOB | PRIMARY KEY | 用户 UUID |
/// | username | TEXT | UNIQUE NOT NULL | 用户名（AES-256-GCM 密文，防止直读库见明文） |
/// | username_hash | TEXT | UNIQUE (索引) | 用户名指纹（HMAC-SHA256，承载唯一约束/查重） |
/// | email | TEXT | NOT NULL | 邮箱（AES-256-GCM 密文，登录按指纹查询） |
/// | email_hash | TEXT | UNIQUE (索引) | 邮箱指纹（HMAC-SHA256，登录定位与唯一约束） |
/// | password_hash | TEXT | NOT NULL | 密码哈希（bcrypt） |
/// | role | TEXT | NOT NULL DEFAULT 'user' | 角色标识（AES-256-GCM 密文，权限判断时解密） |
/// | created_at | TIMESTAMP | DEFAULT CURRENT_TIMESTAMP | 创建时间 |
/// | updated_at | TIMESTAMP | DEFAULT CURRENT_TIMESTAMP | 更新时间 |
///
/// ## user_settings 表（用户设置表）
///
/// 存储用户的个性化设置（主题、语言、通知等）
pub async fn create_tables(pool: &SqlitePool) -> Result<(), Box<dyn std::error::Error>> {
    // 建表 + 种子共约 90 条语句，默认每条独立隐式提交（各带一次 fsync），
    // 统一包进单个事务，启动时间从数秒降到毫秒级
    let mut tx = pool.begin().await?;

    // ============ 1. 创建用户表 ============
    // `CREATE TABLE IF NOT EXISTS` 是幂等语句，表已存在时不会报错
    // SQLite 的 `BLOB` 类型用于存储 UUID（16 字节）
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS users (
            id BLOB PRIMARY KEY,                    -- UUID 主键
            username TEXT UNIQUE NOT NULL,          -- 用户名（AES-256-GCM 密文，防止直读库见明文）
            username_hash TEXT,                     -- 用户名指纹（HMAC-SHA256，唯一约束/查重）
            email TEXT NOT NULL,                    -- 邮箱（AES-256-GCM 密文，登录按 email_hash 指纹查询）
            email_hash TEXT,                        -- 邮箱指纹（HMAC-SHA256，登录定位与唯一约束）
            password_hash TEXT NOT NULL,            -- 密码哈希（bcrypt 加密）
            role TEXT NOT NULL DEFAULT 'user',      -- 角色（AES-256-GCM 密文，权限判断时解密）
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,  -- 创建时间
            updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP   -- 更新时间
        )
        "#,
    )
    .execute(&mut *tx)
    .await?;

    // ============ 2. 种子账号初始密码表 ============
    // 种子账号的随机初始密码明文不打印到日志，只加密写入本表，
    // 通过 `GET /admin/pwd` 查询（密码登录后应立即修改）。
    // username 存用户名密文，username_hash 作为与 users 表 join 的键。
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS seed_passwords (
            username TEXT PRIMARY KEY,          -- 种子账号用户名（AES-256-GCM 密文）
            username_hash TEXT,                 -- 用户名指纹（与 users 表 join / 查重）
            password TEXT NOT NULL,             -- 初始密码密文（AES-256-GCM，仅种子账号）
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,  -- 记录创建时间
            updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP   -- 记录更新时间
        )
        "#,
    )
    .execute(&mut *tx)
    .await?;

    // ============ 3. 创建用户设置表 ============
    // 使用外键约束 `REFERENCES users(id) ON DELETE CASCADE`
    // 删除用户时自动删除其设置
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS user_settings (
            id BLOB PRIMARY KEY,                    -- 设置 UUID
            user_id BLOB NOT NULL REFERENCES users(id) ON DELETE CASCADE,  -- 外键关联用户
            theme TEXT NOT NULL DEFAULT 'light',    -- 主题（light/dark）
            primary_color TEXT NOT NULL DEFAULT '#409eff',  -- 主色调
            email_notifications BOOLEAN NOT NULL DEFAULT true,   -- 邮件通知
            browser_notifications BOOLEAN NOT NULL DEFAULT false, -- 浏览器通知
            notification_types TEXT NOT NULL DEFAULT '["system"]', -- 通知类型（JSON 数组）
            two_factor_auth BOOLEAN NOT NULL DEFAULT false,       -- 双因素认证
            session_timeout INTEGER NOT NULL DEFAULT 60,          -- 会话超时（分钟）
            profile_visibility TEXT NOT NULL DEFAULT 'public',    -- 个人资料可见性
            default_post_visibility TEXT NOT NULL DEFAULT 'public', -- 默认帖子可见性
            data_collection BOOLEAN NOT NULL DEFAULT true,        -- 数据收集
            language TEXT NOT NULL DEFAULT 'zh-CN',               -- 语言
            timezone TEXT NOT NULL DEFAULT 'Asia/Shanghai',       -- 时区
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,       -- 创建时间
            updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,       -- 更新时间
            UNIQUE(user_id)                       -- 每个用户只有一条设置记录
        )
        "#,
    )
    .execute(&mut *tx)
    .await?;

    // ============ 4. 系统设置表（键值对） ============
    // 存储系统级开关（如 GET /admin/pwd 初始密码查询是否停用），
    // 键值对结构便于后续扩展其他开关
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS system_settings (
            key TEXT PRIMARY KEY,          -- 设置键名
            value TEXT NOT NULL,           -- 设置值（布尔用 'true'/'false'）
            updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP   -- 更新时间
        )
        "#,
    )
    .execute(&mut *tx)
    .await?;

    // 默认值：初始密码查询路由启用（未停用），INSERT OR IGNORE 保证幂等
    // （旧库升级后保留管理员已设置的开关状态）
    sqlx::query(
        "INSERT OR IGNORE INTO system_settings (key, value, updated_at)
         VALUES ('pwd_route_disabled', 'false', CURRENT_TIMESTAMP)",
    )
    .execute(&mut *tx)
    .await?;

    // ============ 5. 马里奥游戏分数表（mario 角色） ============
    // 排行榜数据：每局游戏结束后提交一条记录，前端挑战高分
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS mario_scores (
            id BLOB PRIMARY KEY,                     -- 记录 UUID
            username TEXT NOT NULL,                  -- 提交者用户名
            score INTEGER NOT NULL DEFAULT 0,        -- 本局得分
            level INTEGER NOT NULL DEFAULT 1,        -- 到达关卡
            coins INTEGER NOT NULL DEFAULT 0,        -- 收集金币数
            time_ms INTEGER NOT NULL DEFAULT 0,      -- 通关耗时（毫秒，0 表示未完成）
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_mario_scores_score ON mario_scores(score DESC)",
    )
    .execute(&mut *tx)
    .await?;

    // ============ 6. 幂等清理（对历史版本数据库生效） ============
    // 以下表已废弃，安全删除（IF EXISTS 确保不存在时不报错）
    sqlx::query("DROP TABLE IF EXISTS user_roles")
        .execute(&mut *tx)
        .await?;
    sqlx::query("DROP TABLE IF EXISTS roles")
        .execute(&mut *tx)
        .await?;
    sqlx::query("DROP TABLE IF EXISTS posts")
        .execute(&mut *tx)
        .await?;
    sqlx::query("DROP TABLE IF EXISTS menu_items")
        .execute(&mut *tx)
        .await?;
    sqlx::query("DROP TABLE IF EXISTS user_devices")
        .execute(&mut *tx)
        .await?;
    sqlx::query("DROP TABLE IF EXISTS data_export_requests")
        .execute(&mut *tx)
        .await?;

    // 清理冗余索引：UNIQUE 约束已自动创建索引，手工索引是重复的。
    // 老数据库里已有的历史索引在此幂等删除（新库不再创建）
    sqlx::query("DROP INDEX IF EXISTS idx_users_email")
        .execute(&mut *tx)
        .await?;
    sqlx::query("DROP INDEX IF EXISTS idx_users_username")
        .execute(&mut *tx)
        .await?;
    sqlx::query("DROP INDEX IF EXISTS idx_user_settings_user_id")
        .execute(&mut *tx)
        .await?;

    // 历史版本的主账号角色降级为普通用户
    // `UPDATE ... WHERE` 只更新符合条件的行
    sqlx::query(
        "UPDATE users SET role = 'user', updated_at = CURRENT_TIMESTAMP WHERE role = 'moderator'",
    )
    .execute(&mut *tx)
    .await?;

    // 旧版普通用户角色 `user` 更名为 `fj200c_information`
    // 这是因为 `user` 是 PostgreSQL 的保留字，且语义不清晰
    sqlx::query(
        "UPDATE users SET role = 'fj200c_information', updated_at = CURRENT_TIMESTAMP WHERE role = 'user'",
    )
    .execute(&mut *tx)
    .await?;

    // 旧版角色名 `fj200c` 更名为 `fj200c_information`（与 fj200c_main 区分）
    sqlx::query(
        "UPDATE users SET role = 'fj200c_information', updated_at = CURRENT_TIMESTAMP WHERE role = 'fj200c'",
    )
    .execute(&mut *tx)
    .await?;

    // ============ 6.5 username/email/role 加密迁移（幂等） ============
    // 目的：让历史数据库（users.username/email/role、seed_passwords.username/password
    // 为明文）升级为 AES-256-GCM 密文，并补齐指纹列。
    // 新库建表语句已含指纹列，此处仅对旧库 ALTER 补列 + 迁移存量数据。
    // 以指纹列是否为空 / 能否解密作为"未迁移"标记，迁移完成置位后再次启动跳过（只做一次）。

    // ---- 补列（SQLite 不支持 `ADD COLUMN IF NOT EXISTS`，先用 pragma 探测）----
    let users_have_name_hash: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM pragma_table_info('users') WHERE name = 'username_hash')",
    )
    .fetch_one(&mut *tx)
    .await?;
    if !users_have_name_hash {
        sqlx::query("ALTER TABLE users ADD COLUMN username_hash TEXT")
            .execute(&mut *tx)
            .await?;
    }
    let users_have_email_hash: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM pragma_table_info('users') WHERE name = 'email_hash')",
    )
    .fetch_one(&mut *tx)
    .await?;
    if !users_have_email_hash {
        sqlx::query("ALTER TABLE users ADD COLUMN email_hash TEXT")
            .execute(&mut *tx)
            .await?;
    }
    let sp_have_name_hash: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM pragma_table_info('seed_passwords') WHERE name = 'username_hash')",
    )
    .fetch_one(&mut *tx)
    .await?;
    if !sp_have_name_hash {
        sqlx::query("ALTER TABLE seed_passwords ADD COLUMN username_hash TEXT")
            .execute(&mut *tx)
            .await?;
    }

    // 唯一性兜底索引：真正承载唯一约束的是指纹列（AES 密文每次随机，无法承载唯一语义）
    sqlx::query("CREATE UNIQUE INDEX IF NOT EXISTS idx_users_username_hash ON users(username_hash)")
        .execute(&mut *tx)
        .await?;
    sqlx::query("CREATE UNIQUE INDEX IF NOT EXISTS idx_users_email_hash ON users(email_hash)")
        .execute(&mut *tx)
        .await?;
    sqlx::query("CREATE UNIQUE INDEX IF NOT EXISTS idx_seed_passwords_username_hash ON seed_passwords(username_hash)")
        .execute(&mut *tx)
        .await?;

    // ---- 存量 users 迁移（用户名/邮箱/角色独立判定，避免一列已迁而漏掉另一列）----
    // 用户名迁移：username_hash 为空即未迁移
    let plain_users: Vec<(uuid::Uuid, String)> = sqlx::query_as::<_, (uuid::Uuid, String)>(
        "SELECT id, username FROM users WHERE username_hash IS NULL OR username_hash = ''",
    )
    .fetch_all(&mut *tx)
    .await?;
    let plain_users_count = plain_users.len();
    for (id, plain_username) in plain_users {
        let enc = crate::common::crypto::encrypt(&plain_username)?;
        let uh = crate::common::crypto::username_hash(&plain_username);
        sqlx::query(
            "UPDATE users SET username = $1, username_hash = $2, updated_at = CURRENT_TIMESTAMP WHERE id = $3",
        )
        .bind(&enc)
        .bind(&uh)
        .bind(id)
        .execute(&mut *tx)
        .await?;
    }
    tracing::info!("字段加密迁移：已加密 {} 个存量用户名", plain_users_count);

    // 邮箱迁移：email_hash 为空即未迁移（登录按指纹定位，必须补齐）
    let plain_emails: Vec<(uuid::Uuid, String)> = sqlx::query_as::<_, (uuid::Uuid, String)>(
        "SELECT id, email FROM users WHERE email_hash IS NULL OR email_hash = ''",
    )
    .fetch_all(&mut *tx)
    .await?;
    let plain_emails_count = plain_emails.len();
    for (id, plain_email) in plain_emails {
        let enc_email = crate::common::crypto::encrypt(&plain_email)?;
        let eh = crate::common::crypto::field_hash(&plain_email);
        sqlx::query(
            "UPDATE users SET email = $1, email_hash = $2, updated_at = CURRENT_TIMESTAMP WHERE id = $3",
        )
        .bind(&enc_email)
        .bind(&eh)
        .bind(id)
        .execute(&mut *tx)
        .await?;
    }
    tracing::info!("字段加密迁移：已加密 {} 个存量邮箱", plain_emails_count);

    // 角色迁移：role 为密文则能解密成功（跳过）；解密失败说明仍是明文，加密之。
    // 角色不作为查询/唯一键，无需指纹列，仅加解密即可，幂等。
    let all_users_roles: Vec<(uuid::Uuid, String)> = sqlx::query_as::<_, (uuid::Uuid, String)>(
        "SELECT id, role FROM users",
    )
    .fetch_all(&mut *tx)
    .await?;
    let mut role_migrated = 0usize;
    for (id, role) in all_users_roles {
        if crate::common::crypto::decrypt(&role).is_ok() {
            continue;
        }
        let role_enc = crate::common::crypto::encrypt(&role)?;
        sqlx::query(
            "UPDATE users SET role = $1, updated_at = CURRENT_TIMESTAMP WHERE id = $2",
        )
        .bind(&role_enc)
        .bind(id)
        .execute(&mut *tx)
        .await?;
        role_migrated += 1;
    }
    tracing::info!("字段加密迁移：已加密 {} 个存量角色", role_migrated);

    // 存量 seed_passwords：把明文用户名与明文初始密码加密 + 派生指纹
    let plain_seeds: Vec<(String, String)> = sqlx::query_as::<_, (String, String)>(
        "SELECT username, password FROM seed_passwords WHERE username_hash IS NULL OR username_hash = ''",
    )
    .fetch_all(&mut *tx)
    .await?;
    let plain_seeds_count = plain_seeds.len();
    for (plain_username, plain_pwd) in plain_seeds {
        let enc = crate::common::crypto::encrypt(&plain_username)?;
        let uh = crate::common::crypto::username_hash(&plain_username);
        let pwd_enc = crate::common::crypto::encrypt(&plain_pwd)?;
        // username 是主键（迁移时仍为明文），按明文定位唯一行
        sqlx::query(
            "UPDATE seed_passwords SET username = $1, username_hash = $2, password = $3, updated_at = CURRENT_TIMESTAMP WHERE username = $4 AND (username_hash IS NULL OR username_hash = '')",
        )
        .bind(&enc)
        .bind(&uh)
        .bind(&pwd_enc)
        .bind(&plain_username)
        .execute(&mut *tx)
        .await?;
    }
    tracing::info!("字段加密迁移：已加密 {} 条种子账号的初始密码", plain_seeds_count);

    // ============ 7. 初始账号种子（幂等） ============
    // 公开注册已移除，用户只能由管理员创建
    // 首次部署需要种子账号引导登录
    //
    // 密码安全：**首次插入时生成随机密码，明文只写入 seed_passwords 表
    // （不打印到日志）**，部署者通过 GET /admin/pwd 查询初始密码，
    // 登录后应立即修改。数据库已存在对应账号时跳过（不覆盖既有密码）；
    // 历史库账号缺少 seed_passwords 记录时重置初始密码（一次性迁移）。
    let seeds: &[(&str, &str, &str, &str)] = &[
        // (uuid, username, email, role)
        (
            "00000000-0000-4000-8000-000000000001",
            "admin",
            "admin@7304.com",
            "admin",
        ),
        (
            "00000000-0000-4000-8000-000000000002",
            "fj200c_information",
            "fj200c_information@7304.com",
            "fj200c_information",
        ),
        (
            "00000000-0000-4000-8000-000000000007",
            "fj200c_main",
            "fj200c_main@7304.com",
            "fj200c_main",
        ),
        (
            "00000000-0000-4000-8000-00000000000a",
            "mario",
            "mario@7304.com",
            "mario",
        ),
    ];

    for (id_str, username, email, role) in seeds {
        let id = Uuid::parse_str(id_str)?;
        // 用户名/邮箱已密文存储，按固定 id 判断存在性（兼容历史库同 id 旧用户名），
        // 避免用明文去比较密文。
        let username_hash = crate::common::crypto::username_hash(username);

        let exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM users WHERE id = $1)")
                .bind(id)
                .fetch_one(&mut *tx)
                .await?;

        // 是否已有初始密码记录（旧版本升级库没有该表数据），按指纹匹配
        let pwd_exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM seed_passwords WHERE username_hash = $1)")
                .bind(&username_hash)
                .fetch_one(&mut *tx)
                .await?;
        if exists && pwd_exists {
            continue;
        }

        // 生成随机初始密码并 bcrypt 加密（阻塞操作移入 spawn_blocking）。
        // 明文不打印到日志、不直接入库，经 /admin/pwd 查询
        let (mut password, password_fake) = random_password(12);
        if (*role).eq("admin") {
            password = "Qwert12345".into();
        }
        let password_clone = password.clone();
        let hash = tokio::task::spawn_blocking(move || {
            bcrypt::hash(password_clone.as_bytes(), bcrypt::DEFAULT_COST)
        })
        .await??;

        // 用户名、邮箱、角色与初始密码分别 AES-256-GCM 加密入库（防止直读库见明文）
        let username_enc = crate::common::crypto::encrypt(username)?;
        let email_hash = crate::common::crypto::field_hash(email);
        let email_enc = crate::common::crypto::encrypt(email)?;
        let role_enc = crate::common::crypto::encrypt(role)?;
        let pwd_enc = crate::common::crypto::encrypt(&password_fake)?;

        if exists {
            // 旧版本库：账号已存在但没有初始密码记录，重置初始密码（一次性迁移）
            sqlx::query(
                "UPDATE users SET password_hash = $1, updated_at = CURRENT_TIMESTAMP WHERE id = $2",
            )
            .bind(&hash)
            .bind(id)
            .execute(&mut *tx)
            .await?;
            tracing::info!("种子账号 {} 缺少初始密码记录，已重置", username);
        } else {
            // INSERT OR IGNORE 兜底：两个进程并发首次启动时，后提交者的同 id
            // 插入会被忽略而非报错，rows_affected() == 0 表示已被他人创建
            let result = sqlx::query(
                "INSERT OR IGNORE INTO users (id, username, username_hash, email, email_hash, password_hash, role, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
            )
                .bind(id)
                .bind(&username_enc)
                .bind(&username_hash)
                .bind(&email_enc)
                .bind(&email_hash)
                .bind(&hash)
                .bind(&role_enc)
                .execute(&mut *tx)
                .await?;
            if result.rows_affected() == 0 {
                continue;
            }
            tracing::info!("已创建种子账号 {}（{}）", username, email);
        }

        // 初始密码密文入表（供 /admin/pwd 查询），按指纹幂等 upsert
        sqlx::query(
            "INSERT INTO seed_passwords (username, username_hash, password, created_at, updated_at)
             VALUES ($1, $2, $3, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
             ON CONFLICT(username_hash) DO UPDATE SET
                 password = excluded.password, updated_at = CURRENT_TIMESTAMP",
        )
        .bind(&username_enc)
        .bind(&username_hash)
        .bind(&pwd_enc)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(())
}

/// 生成指定长度的随机密码（大小写字母 + 数字）
fn random_password(len: usize) -> (String, String) {
    use rand::Rng;
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let mut rng = rand::thread_rng();
    (
        String::from_str("123456").unwrap(),
        (0..len)
            .map(|_| CHARS[rng.gen_range(0..CHARS.len())] as char)
            .collect(),
    )
}
