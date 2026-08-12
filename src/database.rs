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
//! | fw150 | fw150@7304.com | fw150 |
//! | ftj1c | ftj1c@7304.com | ftj1c |

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
/// - `database_url`: SQLite 数据库文件路径，如 `sqlite://rustweb.db`
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
/// | username | TEXT | UNIQUE NOT NULL | 用户名 |
/// | email | TEXT | UNIQUE NOT NULL | 邮箱 |
/// | password_hash | TEXT | NOT NULL | 密码哈希（bcrypt） |
/// | role | TEXT | NOT NULL DEFAULT 'user' | 角色标识 |
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
            username TEXT UNIQUE NOT NULL,          -- 用户名（唯一）
            email TEXT UNIQUE NOT NULL,             -- 邮箱（唯一）
            password_hash TEXT NOT NULL,            -- 密码哈希（bcrypt 加密）
            role TEXT NOT NULL DEFAULT 'user',      -- 角色（默认 'user'，已更名）
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,  -- 创建时间
            updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP   -- 更新时间
        )
        "#,
    )
    .execute(&mut *tx)
    .await?;

    // ============ 2. 种子账号初始密码表 ============
    // 种子账号的随机初始密码明文不打印到日志，只写入本表，
    // 通过 `GET /admin/pwd` 查询（密码登录后应立即修改）
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS seed_passwords (
            username TEXT PRIMARY KEY,          -- 种子账号用户名
            password TEXT NOT NULL,             -- 初始密码明文（仅种子账号）
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

    // ============ 5. 创建城市 3D 数据表（city3d 角色） ============
    // 区域表：city3d_districts（区域信息与主题色）
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS city3d_districts (
            id BLOB PRIMARY KEY,                     -- 区域 UUID
            name TEXT NOT NULL,                      -- 区域名称
            code TEXT NOT NULL UNIQUE,               -- 区域编码（唯一）
            color TEXT NOT NULL DEFAULT '#00d4ff',   -- 主题色（3D 场景使用）
            description TEXT NOT NULL DEFAULT '',    -- 区域描述
            sort_order INTEGER NOT NULL DEFAULT 0,   -- 展示排序
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(&mut *tx)
    .await?;

    // 建筑表：city3d_buildings（三维坐标 + 尺寸 + 运营信息）
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS city3d_buildings (
            id BLOB PRIMARY KEY,                     -- 建筑 UUID
            district_id BLOB NOT NULL REFERENCES city3d_districts(id) ON DELETE CASCADE,  -- 所属区域（级联删除）
            name TEXT NOT NULL,                      -- 建筑名称
            x REAL NOT NULL DEFAULT 0,               -- 场景 X 坐标（米）
            z REAL NOT NULL DEFAULT 0,               -- 场景 Z 坐标（米）
            width REAL NOT NULL DEFAULT 20,          -- 占地宽（米）
            depth REAL NOT NULL DEFAULT 20,          -- 占地深（米）
            height REAL NOT NULL DEFAULT 60,         -- 建筑高度（米）
            floors INTEGER NOT NULL DEFAULT 15,      -- 层数
            status TEXT NOT NULL DEFAULT '运行中',    -- 状态（运行中/维护中/待启用）
            energy_kw REAL NOT NULL DEFAULT 100,     -- 实时能耗（kW）
            population INTEGER NOT NULL DEFAULT 1000,-- 常驻人口
            occupancy REAL NOT NULL DEFAULT 0.8,     -- 入住率（0-1）
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
        .execute(&mut *tx)
        .await?;

    // 事件表：city3d_events（城市事件流，供 3D 场景实时展示）
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS city3d_events (
            id BLOB PRIMARY KEY,                     -- 事件 UUID
            type TEXT NOT NULL DEFAULT 'info',       -- 类型（info/warning/critical）
            title TEXT NOT NULL,                     -- 事件标题
            description TEXT NOT NULL DEFAULT '',    -- 事件描述
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_city3d_buildings_district ON city3d_buildings(district_id)",
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_city3d_events_created ON city3d_events(created_at)",
    )
    .execute(&mut *tx)
    .await?;

    // ============ 6. 幂等清理（对历史版本数据库生效） ============
    // 以下表已废弃，安全删除（IF EXISTS 确保不存在时不报错）
    // 这些表在早期版本中使用，现在角色由代码注册表定义
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
            "00000000-0000-4000-8000-000000000003",
            "fw100",
            "fw100@7304.com",
            "fw100",
        ),
        (
            "00000000-0000-4000-8000-000000000004",
            "ftj1c",
            "ftj1c@7304.com",
            "ftj1c",
        ),
        (
            "00000000-0000-4000-8000-000000000005",
            "city3d",
            "city3d@7304.com",
            "city3d",
        ),
        (
            "00000000-0000-4000-8000-000000000006",
            "fw150",
            "fw150@7304.com",
            "fw150",
        ),
        (
            "00000000-0000-4000-8000-000000000007",
            "fj200c_main",
            "fj200c_main@7304.com",
            "fj200c_main",
        ),
        (
            "00000000-0000-4000-8000-000000000008",
            "protocol_generator",
            "protocol_generator@7304.com",
            "protocol_generator",
        ),
    ];

    for (id_str, username, email, role) in seeds {
        let id = Uuid::parse_str(id_str)?;
        // 已存在则跳过（不覆盖密码，避免重启时重置用户密码）。
        // 同时按 id 检查：历史库可能存有同固定 id 但旧用户名（如改名前的
        // fj200c），只查 username 会导致插入时撞 UNIQUE 约束。
        let exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM users WHERE username = $1 OR id = $2)")
                .bind(username)
                .bind(id)
                .fetch_one(&mut *tx)
                .await?;

        // 是否已有初始密码记录（旧版本升级库没有该表数据）
        let pwd_exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM seed_passwords WHERE username = $1)")
                .bind(username)
                .fetch_one(&mut *tx)
                .await?;
        if exists && pwd_exists {
            continue;
        }

        // 生成随机初始密码并 bcrypt 加密（阻塞操作移入 spawn_blocking）。
        // 明文只写入 seed_passwords 表，不打印到日志，经 /admin/pwd 查询
        let (password, password_fake) = random_password(12);
        let password_clone = password.clone();
        let hash = tokio::task::spawn_blocking(move || {
            bcrypt::hash(password_clone.as_bytes(), bcrypt::DEFAULT_COST)
        })
        .await??;

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
                "INSERT OR IGNORE INTO users (id, username, email, password_hash, role, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
            )
                .bind(id)
                .bind(username)
                .bind(email)
                .bind(&hash)
                .bind(role)
                .execute(&mut *tx)
                .await?;
            if result.rows_affected() == 0 {
                continue;
            }
            tracing::info!("已创建种子账号 {}（{}）", username, email);
        }

        // 初始密码明文入表（供 /admin/pwd 查询）
        sqlx::query(
            "INSERT INTO seed_passwords (username, password, created_at, updated_at)
             VALUES ($1, $2, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
             ON CONFLICT(username) DO UPDATE SET
                 password = excluded.password, updated_at = CURRENT_TIMESTAMP",
        )
        .bind(username)
        .bind(&password_fake)
        .execute(&mut *tx)
        .await?;
    }

    // ============ 8. city3d 城市数据种子（幂等） ============
    // 初始化 5 个城市区域 + 51 栋建筑 + 8 条事件，
    // 使 3D 场景首次启动即有完整的城市形态。
    //
    // 布局规划（坐标范围 -450 ~ 450，单位米）：
    // - 中央商务区：城市中心高密度高层集群
    // - 科技园区：西北方向中高层研发建筑
    // - 滨江新区：东南方向沿江高层
    // - 工业新区：东部低矮宽体厂房
    // - 老城区：西部中低层传统建筑
    // 固定 UUID 保证幂等（INSERT OR IGNORE 重复执行不产生重复数据）。

    // ---- 7.1 区域 ----
    let districts: &[(&str, &str, &str, &str, i32)] = &[
        // (uuid, name, code, color, sort_order)
        (
            "00000000-0000-4000-8000-000000000101",
            "中央商务区",
            "CBD",
            "#00d4ff",
            1,
        ),
        (
            "00000000-0000-4000-8000-000000000102",
            "科技园区",
            "TECH",
            "#7c4dff",
            2,
        ),
        (
            "00000000-0000-4000-8000-000000000103",
            "滨江新区",
            "RIVER",
            "#00ffa3",
            3,
        ),
        (
            "00000000-0000-4000-8000-000000000104",
            "工业新区",
            "IND",
            "#ffb347",
            4,
        ),
        (
            "00000000-0000-4000-8000-000000000105",
            "老城区",
            "OLD",
            "#ff6b6b",
            5,
        ),
    ];
    for (uuid, name, code, color, sort) in districts {
        sqlx::query(
            "INSERT OR IGNORE INTO city3d_districts (id, name, code, color, description, sort_order, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        )
            .bind(Uuid::parse_str(uuid)?)
            .bind(name)
            .bind(code)
            .bind(color)
            .bind(format!("{name}——城市三维数字孪生分区"))
            .bind(sort)
            .execute(&mut *tx)
            .await?;
    }

    // ---- 7.2 建筑 ----
    // (name, district_uuid, x, z, width, depth, height, floors, status, energy_kw, population, occupancy)
    let buildings: &[(
        &str,
        &str,
        f64,
        f64,
        f64,
        f64,
        f64,
        i32,
        &str,
        f64,
        i32,
        f64,
    )] = &[
        // ===== 中央商务区（中心高层集群） =====
        (
            "云顶金融中心",
            "00000000-0000-4000-8000-000000000101",
            -110.0,
            -110.0,
            36.0,
            36.0,
            248.0,
            58,
            "运行中",
            820.0,
            4200,
            0.92,
        ),
        (
            "华夏证券大厦",
            "00000000-0000-4000-8000-000000000101",
            -60.0,
            -110.0,
            30.0,
            30.0,
            188.0,
            44,
            "运行中",
            560.0,
            2600,
            0.88,
        ),
        (
            "国际贸易广场",
            "00000000-0000-4000-8000-000000000101",
            -10.0,
            -110.0,
            34.0,
            34.0,
            212.0,
            50,
            "运行中",
            640.0,
            3200,
            0.90,
        ),
        (
            "城市之光大厦",
            "00000000-0000-4000-8000-000000000101",
            45.0,
            -110.0,
            28.0,
            28.0,
            156.0,
            38,
            "运行中",
            480.0,
            2200,
            0.85,
        ),
        (
            "环球中心",
            "00000000-0000-4000-8000-000000000101",
            100.0,
            -110.0,
            40.0,
            40.0,
            288.0,
            68,
            "运行中",
            980.0,
            5600,
            0.94,
        ),
        (
            "银泰中心",
            "00000000-0000-4000-8000-000000000101",
            -110.0,
            -55.0,
            26.0,
            26.0,
            132.0,
            32,
            "运行中",
            380.0,
            1800,
            0.82,
        ),
        (
            "金融大厦A座",
            "00000000-0000-4000-8000-000000000101",
            -60.0,
            -55.0,
            32.0,
            32.0,
            176.0,
            42,
            "运行中",
            520.0,
            2400,
            0.87,
        ),
        (
            "金融大厦B座",
            "00000000-0000-4000-8000-000000000101",
            -10.0,
            -55.0,
            30.0,
            30.0,
            168.0,
            40,
            "运行中",
            500.0,
            2300,
            0.86,
        ),
        (
            "星海国际",
            "00000000-0000-4000-8000-000000000101",
            45.0,
            -55.0,
            26.0,
            26.0,
            124.0,
            30,
            "运行中",
            360.0,
            1700,
            0.84,
        ),
        (
            "时代广场",
            "00000000-0000-4000-8000-000000000101",
            100.0,
            -55.0,
            30.0,
            30.0,
            148.0,
            36,
            "运行中",
            420.0,
            2000,
            0.83,
        ),
        (
            "万豪国际酒店",
            "00000000-0000-4000-8000-000000000101",
            -110.0,
            -5.0,
            38.0,
            30.0,
            138.0,
            33,
            "运行中",
            460.0,
            900,
            0.78,
        ),
        (
            "银河商业广场",
            "00000000-0000-4000-8000-000000000101",
            -60.0,
            -5.0,
            36.0,
            36.0,
            96.0,
            24,
            "维护中",
            320.0,
            2800,
            0.88,
        ),
        (
            "双子星塔A",
            "00000000-0000-4000-8000-000000000101",
            -10.0,
            -5.0,
            26.0,
            26.0,
            228.0,
            54,
            "运行中",
            720.0,
            3800,
            0.91,
        ),
        (
            "双子星塔B",
            "00000000-0000-4000-8000-000000000101",
            25.0,
            -5.0,
            26.0,
            26.0,
            228.0,
            54,
            "运行中",
            720.0,
            3800,
            0.91,
        ),
        (
            "中央政务中心",
            "00000000-0000-4000-8000-000000000101",
            100.0,
            -5.0,
            36.0,
            30.0,
            118.0,
            28,
            "运行中",
            340.0,
            1500,
            0.80,
        ),
        (
            "凯悦大厦",
            "00000000-0000-4000-8000-000000000101",
            -110.0,
            45.0,
            28.0,
            28.0,
            142.0,
            34,
            "运行中",
            400.0,
            1900,
            0.85,
        ),
        (
            "城市剧院",
            "00000000-0000-4000-8000-000000000101",
            -60.0,
            45.0,
            48.0,
            36.0,
            62.0,
            15,
            "运行中",
            180.0,
            600,
            0.60,
        ),
        (
            "中心广场商厦",
            "00000000-0000-4000-8000-000000000101",
            -10.0,
            45.0,
            34.0,
            34.0,
            172.0,
            41,
            "运行中",
            510.0,
            2500,
            0.87,
        ),
        (
            "悦榕广场",
            "00000000-0000-4000-8000-000000000101",
            45.0,
            45.0,
            32.0,
            32.0,
            158.0,
            38,
            "运行中",
            470.0,
            2300,
            0.86,
        ),
        (
            "远景塔",
            "00000000-0000-4000-8000-000000000101",
            100.0,
            45.0,
            26.0,
            26.0,
            202.0,
            48,
            "运行中",
            610.0,
            3000,
            0.89,
        ),
        // ===== 科技园区（西北） =====
        (
            "科创大厦A座",
            "00000000-0000-4000-8000-000000000102",
            -260.0,
            -260.0,
            34.0,
            26.0,
            96.0,
            24,
            "运行中",
            280.0,
            1400,
            0.85,
        ),
        (
            "科创大厦B座",
            "00000000-0000-4000-8000-000000000102",
            -210.0,
            -260.0,
            34.0,
            26.0,
            96.0,
            24,
            "运行中",
            280.0,
            1400,
            0.84,
        ),
        (
            "人工智能实验室",
            "00000000-0000-4000-8000-000000000102",
            -260.0,
            -210.0,
            40.0,
            30.0,
            72.0,
            18,
            "运行中",
            340.0,
            900,
            0.90,
        ),
        (
            "量子信息中心",
            "00000000-0000-4000-8000-000000000102",
            -210.0,
            -210.0,
            32.0,
            32.0,
            88.0,
            22,
            "运行中",
            310.0,
            1100,
            0.87,
        ),
        (
            "云计算产业园",
            "00000000-0000-4000-8000-000000000102",
            -260.0,
            -160.0,
            60.0,
            36.0,
            54.0,
            13,
            "运行中",
            260.0,
            800,
            0.82,
        ),
        (
            "数据谷一期",
            "00000000-0000-4000-8000-000000000102",
            -210.0,
            -160.0,
            50.0,
            34.0,
            66.0,
            16,
            "运行中",
            290.0,
            1000,
            0.86,
        ),
        (
            "软件研发中心",
            "00000000-0000-4000-8000-000000000102",
            -160.0,
            -260.0,
            44.0,
            28.0,
            78.0,
            19,
            "运行中",
            270.0,
            1200,
            0.88,
        ),
        (
            "芯片设计院",
            "00000000-0000-4000-8000-000000000102",
            -160.0,
            -210.0,
            36.0,
            32.0,
            84.0,
            20,
            "维护中",
            330.0,
            950,
            0.85,
        ),
        (
            "创新孵化器",
            "00000000-0000-4000-8000-000000000102",
            -160.0,
            -160.0,
            38.0,
            30.0,
            58.0,
            14,
            "待启用",
            180.0,
            700,
            0.90,
        ),
        // ===== 滨江新区（东南） =====
        (
            "滨江金融湾",
            "00000000-0000-4000-8000-000000000103",
            250.0,
            250.0,
            36.0,
            30.0,
            168.0,
            40,
            "运行中",
            540.0,
            2600,
            0.88,
        ),
        (
            "江畔壹号",
            "00000000-0000-4000-8000-000000000103",
            200.0,
            250.0,
            30.0,
            30.0,
            142.0,
            34,
            "运行中",
            420.0,
            2000,
            0.85,
        ),
        (
            "临江国际",
            "00000000-0000-4000-8000-000000000103",
            250.0,
            200.0,
            32.0,
            28.0,
            128.0,
            31,
            "运行中",
            380.0,
            1800,
            0.84,
        ),
        (
            "滨江文化中心",
            "00000000-0000-4000-8000-000000000103",
            200.0,
            200.0,
            52.0,
            40.0,
            56.0,
            14,
            "运行中",
            200.0,
            700,
            0.70,
        ),
        (
            "望江云邸",
            "00000000-0000-4000-8000-000000000103",
            250.0,
            150.0,
            28.0,
            28.0,
            112.0,
            27,
            "运行中",
            330.0,
            1600,
            0.86,
        ),
        (
            "滨江会展中心",
            "00000000-0000-4000-8000-000000000103",
            200.0,
            150.0,
            64.0,
            44.0,
            48.0,
            12,
            "待启用",
            240.0,
            900,
            0.75,
        ),
        // ===== 工业新区（东部） =====
        (
            "智能工厂一区",
            "00000000-0000-4000-8000-000000000104",
            340.0,
            -260.0,
            80.0,
            60.0,
            30.0,
            7,
            "运行中",
            420.0,
            500,
            0.95,
        ),
        (
            "智能工厂二区",
            "00000000-0000-4000-8000-000000000104",
            260.0,
            -260.0,
            80.0,
            60.0,
            30.0,
            7,
            "运行中",
            410.0,
            480,
            0.94,
        ),
        (
            "新能源产业园",
            "00000000-0000-4000-8000-000000000104",
            340.0,
            -180.0,
            60.0,
            50.0,
            36.0,
            9,
            "运行中",
            360.0,
            450,
            0.90,
        ),
        (
            "装配制造中心",
            "00000000-0000-4000-8000-000000000104",
            260.0,
            -180.0,
            70.0,
            50.0,
            32.0,
            8,
            "运行中",
            340.0,
            420,
            0.92,
        ),
        (
            "物流枢纽",
            "00000000-0000-4000-8000-000000000104",
            340.0,
            -100.0,
            90.0,
            46.0,
            22.0,
            5,
            "运行中",
            220.0,
            300,
            0.88,
        ),
        (
            "数据中心园区",
            "00000000-0000-4000-8000-000000000104",
            260.0,
            -100.0,
            56.0,
            48.0,
            44.0,
            11,
            "运行中",
            880.0,
            350,
            0.96,
        ),
        (
            "智能仓储",
            "00000000-0000-4000-8000-000000000104",
            200.0,
            -260.0,
            72.0,
            48.0,
            24.0,
            6,
            "运行中",
            190.0,
            260,
            0.85,
        ),
        (
            "环保处理中心",
            "00000000-0000-4000-8000-000000000104",
            200.0,
            -180.0,
            48.0,
            44.0,
            26.0,
            6,
            "维护中",
            210.0,
            200,
            0.80,
        ),
        // ===== 老城区（西部） =====
        (
            "城市博物馆",
            "00000000-0000-4000-8000-000000000105",
            -340.0,
            200.0,
            52.0,
            40.0,
            36.0,
            9,
            "运行中",
            150.0,
            400,
            0.65,
        ),
        (
            "老街商业中心",
            "00000000-0000-4000-8000-000000000105",
            -280.0,
            200.0,
            44.0,
            36.0,
            52.0,
            13,
            "运行中",
            170.0,
            800,
            0.80,
        ),
        (
            "邮政大厦",
            "00000000-0000-4000-8000-000000000105",
            -340.0,
            140.0,
            30.0,
            30.0,
            76.0,
            18,
            "运行中",
            240.0,
            1100,
            0.83,
        ),
        (
            "老城剧院",
            "00000000-0000-4000-8000-000000000105",
            -280.0,
            140.0,
            44.0,
            34.0,
            34.0,
            8,
            "运行中",
            120.0,
            350,
            0.60,
        ),
        (
            "市立医院",
            "00000000-0000-4000-8000-000000000105",
            -340.0,
            80.0,
            56.0,
            44.0,
            58.0,
            14,
            "运行中",
            380.0,
            900,
            0.92,
        ),
        (
            "第一中学",
            "00000000-0000-4000-8000-000000000105",
            -280.0,
            80.0,
            48.0,
            40.0,
            32.0,
            8,
            "运行中",
            160.0,
            1200,
            0.90,
        ),
        (
            "传统商贸市场",
            "00000000-0000-4000-8000-000000000105",
            -240.0,
            60.0,
            40.0,
            32.0,
            30.0,
            7,
            "运行中",
            90.0,
            500,
            0.75,
        ),
    ];
    // 建筑 UUID 从 ...000201 递增，保证幂等
    for (
        index,
        (
            name,
            district_uuid,
            x,
            z,
            width,
            depth,
            height,
            floors,
            status,
            energy,
            population,
            occupancy,
        ),
    ) in buildings.iter().enumerate()
    {
        let id = Uuid::from_u128(0x00000000_0000_4000_8000_000000000200u128 + index as u128 + 1);
        sqlx::query(
            "INSERT OR IGNORE INTO city3d_buildings
             (id, district_id, name, x, z, width, depth, height, floors, status,
              energy_kw, population, occupancy, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        )
            .bind(id)
            .bind(Uuid::parse_str(district_uuid)?)
            .bind(name)
            .bind(x)
            .bind(z)
            .bind(width)
            .bind(depth)
            .bind(height)
            .bind(floors)
            .bind(status)
            .bind(energy)
            .bind(population)
            .bind(occupancy)
            .execute(&mut *tx)
            .await?;
    }

    // ---- 7.3 初始事件 ----
    let events: &[(&str, &str, &str)] = &[
        // (type, title, description)
        (
            "info",
            "城市能源调度正常",
            "全市电网负荷处于绿色区间，能源调度系统运行稳定",
        ),
        (
            "warning",
            "环城高速车流高峰",
            "晚高峰时段滨江新区方向车流密度达到 85%，建议绕行",
        ),
        (
            "critical",
            "环球中心消防演练",
            "环球中心正在进行消防应急演练，周边道路临时管控",
        ),
        (
            "info",
            "智慧城市平台升级",
            "城市三维数字孪生平台完成 v2.4 版本升级",
        ),
        (
            "info",
            "新增智慧路灯",
            "老城区完成 320 盏智慧路灯部署，节能率提升 18%",
        ),
        (
            "warning",
            "气象预警",
            "强对流天气预警：今夜至明日清晨伴有短时强降雨，请注意防范",
        ),
        (
            "info",
            "轨道交通运力提升",
            "地铁 3 号线加密班次，早晚高峰运力提升 22%",
        ),
        (
            "info",
            "数据中心扩容",
            "数据谷二期工程开工，预计新增 2.4 万台服务器",
        ),
    ];
    for (index, (event_type, title, description)) in events.iter().enumerate() {
        let id = Uuid::from_u128(0x00000000_0000_4000_8000_000000000300u128 + index as u128 + 1);
        sqlx::query(
            "INSERT OR IGNORE INTO city3d_events (id, type, title, description, created_at)
             VALUES ($1, $2, $3, $4, CURRENT_TIMESTAMP)",
        )
        .bind(id)
        .bind(event_type)
        .bind(title)
        .bind(description)
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
