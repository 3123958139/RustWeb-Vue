//! # 管理员服务模块
//!
//! 提供用户管理的数据库操作：
//!
//! | 服务 | 功能 | SQL 操作 |
//! |------|------|----------|
//! | `list_users` | 获取所有用户 | SELECT ... ORDER BY created_at DESC |
//! | `update_user_role` | 修改用户角色 | UPDATE ... RETURNING * |
//! | `delete_user` | 删除用户 | DELETE ... RETURNING * |
//!
//! # 说明
//!
//! - 创建用户复用 `AuthService::create_user`（唯一入口）
//! - 所有方法返回 `Result<Option<T>>`，`None` 表示记录不存在

use crate::common::models::User;           // 用户结构体
use crate::database::DatabaseConnection;   // 数据库连接池
use chrono::Utc;                           // UTC 时间
use uuid::Uuid;                            // UUID 类型

/// 管理员服务结构体
///
/// 使用结构体组织相关函数（类似 Java 的静态方法类）
pub struct UserAdminService;

impl UserAdminService {
    /// 系统设置键：初始密码查询路由（GET /admin/pwd）是否停用
    const PWD_ROUTE_DISABLED_KEY: &'static str = "pwd_route_disabled";

    /// 查询初始密码查询路由（GET /admin/pwd）是否停用
    ///
    /// 管理员在管理后台勾选后返回 `true`，此时该端点不再处理查询请求。
    /// 默认 `false`（未停用），记录缺失时按未停用处理。
    pub async fn is_pwd_route_disabled(
        pool: &DatabaseConnection,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let value: Option<String> = sqlx::query_scalar(
            "SELECT value FROM system_settings WHERE key = $1",
        )
        .bind(Self::PWD_ROUTE_DISABLED_KEY)
        .fetch_optional(pool)
        .await?;
        Ok(value.as_deref() == Some("true"))
    }

    /// 设置初始密码查询路由（GET /admin/pwd）是否停用
    ///
    /// 使用 `INSERT ... ON CONFLICT DO UPDATE` 保证记录存在时更新、
    /// 不存在时插入（幂等，无需预置默认行）。
    pub async fn set_pwd_route_disabled(
        pool: &DatabaseConnection,
        disabled: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        sqlx::query(
            "INSERT INTO system_settings (key, value, updated_at)
             VALUES ($1, $2, CURRENT_TIMESTAMP)
             ON CONFLICT(key) DO UPDATE SET
                 value = excluded.value, updated_at = CURRENT_TIMESTAMP",
        )
        .bind(Self::PWD_ROUTE_DISABLED_KEY)
        .bind(if disabled { "true" } else { "false" })
        .execute(pool)
        .await?;
        Ok(())
    }

    /// 获取所有用户列表
    ///
    /// # 参数
    /// - `pool`: 数据库连接池
    ///
    /// # 返回值
    /// 按创建时间降序排列的用户列表
    ///
    /// # SQL
    /// ```sql
    /// SELECT id, username, email, role, created_at, updated_at FROM users ORDER BY created_at DESC
    /// ```
    ///
    /// # 语法说明
    ///
    /// - `query_as::<_, User>(SQL)`: 执行 SQL 并映射到 `User` 结构体
    /// - `fetch_all(pool)`: 查询所有匹配的行
    pub async fn list_users(
        pool: &DatabaseConnection,
    ) -> Result<Vec<User>, Box<dyn std::error::Error>> {
        let users = sqlx::query_as::<_, User>(
            // 列表接口不返回 password_hash 列（见 User 的 #[sqlx(default)]）
            "SELECT id, username, email, role, created_at, updated_at FROM users ORDER BY created_at DESC",
        )
        .fetch_all(pool)
        .await?;
        Ok(users)
    }

    /// 修改用户角色
    ///
    /// # 参数
    /// - `pool`: 数据库连接池
    /// - `user_id`: 目标用户 UUID
    /// - `role`: 新角色标识
    ///
    /// # 返回值
    /// - `Ok(Some(User))` - 更新成功
    /// - `Ok(None)` - 用户不存在
    ///
    /// # SQL
    /// ```sql
    /// UPDATE users SET role = $1, updated_at = $2 WHERE id = $3 RETURNING *
    /// ```
    ///
    /// # 语法说明
    ///
    /// `RETURNING *` 是 SQLite 3.35+ 的特性：
    /// - 更新后直接返回修改的行
    /// - 避免额外的 SELECT 查询
    /// - `fetch_optional` 在记录不存在时返回 `None`
    pub async fn update_user_role(
        pool: &DatabaseConnection,
        user_id: Uuid,
        role: &str,
    ) -> Result<Option<User>, Box<dyn std::error::Error>> {
        // 角色加密入库（读取时由 User::FromRow 解密）
        let role_enc = crate::common::crypto::encrypt(role)?;
        let user = sqlx::query_as::<_, User>(
            "UPDATE users SET role = $1, updated_at = $2 WHERE id = $3 RETURNING *",
        )
        .bind(&role_enc)
        .bind(Utc::now())
        .bind(user_id)
        .fetch_optional(pool)
        .await?;
        Ok(user)
    }

    /// 删除用户
    ///
    /// # 参数
    /// - `pool`: 数据库连接池
    /// - `user_id`: 目标用户 UUID
    ///
    /// # 返回值
    /// - `Ok(Some(User))` - 删除成功（返回被删除的用户）
    /// - `Ok(None)` - 用户不存在
    ///
    /// # 注意
    ///
    /// - 外键约束 `ON DELETE CASCADE` 会自动删除关联的 `user_settings`
    /// - 删除操作不可逆
    pub async fn delete_user(
        pool: &DatabaseConnection,
        user_id: Uuid,
    ) -> Result<Option<User>, Box<dyn std::error::Error>> {
        let user = sqlx::query_as::<_, User>(
            "DELETE FROM users WHERE id = $1 RETURNING *",
        )
        .bind(user_id)
        .fetch_optional(pool)
        .await?;
        Ok(user)
    }
}
