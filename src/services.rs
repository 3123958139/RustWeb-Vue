use crate::models::{User, Post, CreateUserRequest, LoginRequest, CreatePostRequest, UserSettings, UpdateSettingsRequest, UserDevice, ExportDataRequest, DeleteAccountRequest, MenuItem, CreateMenuItemRequest, UpdateMenuItemRequest, Role, CreateRoleRequest, UpdateRoleRequest, UpdateUserRoleRequest};
use crate::database::DatabaseConnection;
use bcrypt::{hash, verify, DEFAULT_COST};
use uuid::Uuid;
use chrono::Utc;
use serde_json;

pub struct UserService;

impl UserService {
    pub async fn create_user(
        pool: &DatabaseConnection,
        user_data: CreateUserRequest,
    ) -> Result<User, Box<dyn std::error::Error>> {
        // 检查用户是否已存在
        let existing_user = sqlx::query_as::<_, User>(
            "SELECT * FROM users WHERE email = $1 OR username = $2"
        )
        .bind(&user_data.email)
        .bind(&user_data.username)
        .fetch_optional(pool)
        .await?;
        
        if existing_user.is_some() {
            return Err("用户已存在".into());
        }

        // 加密密码
        let password_hash = hash(user_data.password.as_bytes(), DEFAULT_COST)?;

        // 创建用户
        let user = sqlx::query_as::<_, User>(
            r#"
            INSERT INTO users (username, email, password_hash, role)
            VALUES ($1, $2, $3, $4)
            RETURNING *
            "#,
        )
        .bind(&user_data.username)
        .bind(&user_data.email)
        .bind(&password_hash)
        .bind("user")
        .fetch_one(pool)
        .await?;

        // 创建默认设置
        Self::create_default_settings(pool, user.id).await?;

        Ok(user)
    }

    pub async fn login(
        pool: &DatabaseConnection,
        login_data: LoginRequest,
    ) -> Result<User, Box<dyn std::error::Error>> {
        let user = sqlx::query_as::<_, User>(
            "SELECT * FROM users WHERE email = $1"
        )
        .bind(&login_data.email)
        .fetch_optional(pool)
        .await?
        .ok_or("用户不存在")?;

        // 验证密码
        let is_valid = verify(login_data.password.as_bytes(), &user.password_hash)?;
        if !is_valid {
            return Err("密码错误".into());
        }

        Ok(user)
    }

    pub async fn get_user_by_id(
        pool: &DatabaseConnection,
        user_id: Uuid,
    ) -> Result<Option<User>, Box<dyn std::error::Error>> {
        let user = sqlx::query_as::<_, User>(
            "SELECT * FROM users WHERE id = $1"
        )
        .bind(user_id)
        .fetch_optional(pool)
        .await?;

        Ok(user)
    }

    pub async fn get_all_users(
        pool: &DatabaseConnection,
    ) -> Result<Vec<User>, Box<dyn std::error::Error>> {
        let users = sqlx::query_as::<_, User>(
            "SELECT * FROM users ORDER BY created_at DESC"
        )
        .fetch_all(pool)
        .await?;

        Ok(users)
    }

    pub async fn update_user_role(
        pool: &DatabaseConnection,
        user_id: Uuid,
        role: &str,
    ) -> Result<User, Box<dyn std::error::Error>> {
        let user = sqlx::query_as::<_, User>(
            "UPDATE users SET role = $1, updated_at = $2 WHERE id = $3 RETURNING *"
        )
        .bind(role)
        .bind(Utc::now())
        .bind(user_id)
        .fetch_one(pool)
        .await?;

        Ok(user)
    }

    // 创建设置服务
    pub async fn create_default_settings(
        pool: &DatabaseConnection,
        user_id: Uuid,
    ) -> Result<UserSettings, Box<dyn std::error::Error>> {
        let settings = sqlx::query_as::<_, UserSettings>(
            r#"
            INSERT INTO user_settings (
                user_id, theme, primary_color, email_notifications, 
                browser_notifications, notification_types, two_factor_auth,
                session_timeout, profile_visibility, default_post_visibility,
                data_collection, language, timezone
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            RETURNING *
            "#,
        )
        .bind(user_id)
        .bind("light")
        .bind("#409eff")
        .bind(true)
        .bind(false)
        .bind(serde_json::to_string(&vec!["posts", "system"])?)
        .bind(false)
        .bind(60)
        .bind("public")
        .bind("public")
        .bind(true)
        .bind("zh-CN")
        .bind("Asia/Shanghai")
        .fetch_one(pool)
        .await?;

        Ok(settings)
    }

    pub async fn get_user_settings(
        pool: &DatabaseConnection,
        user_id: Uuid,
    ) -> Result<Option<UserSettings>, Box<dyn std::error::Error>> {
        let settings = sqlx::query_as::<_, UserSettings>(
            "SELECT * FROM user_settings WHERE user_id = $1"
        )
        .bind(user_id)
        .fetch_optional(pool)
        .await?;

        Ok(settings)
    }

    pub async fn update_user_settings(
        pool: &DatabaseConnection,
        user_id: Uuid,
        settings_data: UpdateSettingsRequest,
    ) -> Result<UserSettings, Box<dyn std::error::Error>> {
        // 先检查设置是否存在
        let existing_settings = Self::get_user_settings(pool, user_id).await?;
        
        let settings = if let Some(existing) = existing_settings {
            // 更新现有设置
            sqlx::query_as::<_, UserSettings>(
                r#"
                UPDATE user_settings SET
                    theme = COALESCE($2, theme),
                    primary_color = COALESCE($3, primary_color),
                    email_notifications = COALESCE($4, email_notifications),
                    browser_notifications = COALESCE($5, browser_notifications),
                    notification_types = COALESCE($6, notification_types),
                    two_factor_auth = COALESCE($7, two_factor_auth),
                    session_timeout = COALESCE($8, session_timeout),
                    profile_visibility = COALESCE($9, profile_visibility),
                    default_post_visibility = COALESCE($10, default_post_visibility),
                    data_collection = COALESCE($11, data_collection),
                    language = COALESCE($12, language),
                    timezone = COALESCE($13, timezone),
                    updated_at = $14
                WHERE user_id = $1
                RETURNING *
                "#,
            )
            .bind(user_id)
            .bind(&settings_data.theme)
            .bind(&settings_data.primary_color)
            .bind(&settings_data.email_notifications)
            .bind(&settings_data.browser_notifications)
            .bind(settings_data.notification_types.as_ref().map(|nt| serde_json::to_string(nt).unwrap_or_default()))
            .bind(&settings_data.two_factor_auth)
            .bind(&settings_data.session_timeout)
            .bind(&settings_data.profile_visibility)
            .bind(&settings_data.default_post_visibility)
            .bind(&settings_data.data_collection)
            .bind(&settings_data.language)
            .bind(&settings_data.timezone)
            .bind(Utc::now())
            .fetch_one(pool)
            .await?
        } else {
            // 创建新设置
            Self::create_default_settings(pool, user_id).await?
        };

        Ok(settings)
    }

    pub async fn get_user_devices(
        pool: &DatabaseConnection,
        user_id: Uuid,
    ) -> Result<Vec<UserDevice>, Box<dyn std::error::Error>> {
        let devices = sqlx::query_as::<_, UserDevice>(
            "SELECT * FROM user_devices WHERE user_id = $1 ORDER BY last_login DESC"
        )
        .bind(user_id)
        .fetch_all(pool)
        .await?;

        Ok(devices)
    }

    pub async fn logout_device(
        pool: &DatabaseConnection,
        user_id: Uuid,
        device_id: Uuid,
    ) -> Result<(), Box<dyn std::error::Error>> {
        sqlx::query(
            "DELETE FROM user_devices WHERE id = $1 AND user_id = $2"
        )
        .bind(device_id)
        .bind(user_id)
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn export_user_data(
        pool: &DatabaseConnection,
        user_id: Uuid,
        export_request: ExportDataRequest,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // 这里应该实现数据导出逻辑
        // 包括用户信息、文章、设置等
        // 然后发送邮件给用户
        
        // 暂时只是记录导出请求
        sqlx::query(
            r#"
            INSERT INTO data_export_requests (user_id, email, data_types, status)
            VALUES ($1, $2, $3, 'pending')
            "#,
        )
        .bind(user_id)
        .bind(&export_request.email)
        .bind(serde_json::to_string(&export_request.data_types)?)
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn delete_user_account(
        pool: &DatabaseConnection,
        user_id: Uuid,
        delete_request: DeleteAccountRequest,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // 验证密码
        let user = Self::get_user_by_id(pool, user_id).await?
            .ok_or("用户不存在")?;
        
        let is_valid = verify(delete_request.password.as_bytes(), &user.password_hash)?;
        if !is_valid {
            return Err("密码错误".into());
        }

        // 验证确认信息
        if delete_request.confirmation != "DELETE" {
            return Err("确认信息不正确".into());
        }

        // 删除用户相关数据
        sqlx::query("DELETE FROM user_devices WHERE user_id = $1")
            .bind(user_id)
            .execute(pool)
            .await?;

        sqlx::query("DELETE FROM user_settings WHERE user_id = $1")
            .bind(user_id)
            .execute(pool)
            .await?;

        sqlx::query("DELETE FROM posts WHERE author_id = $1")
            .bind(user_id)
            .execute(pool)
            .await?;

        sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(user_id)
            .execute(pool)
            .await?;

        Ok(())
    }

    // 菜单管理相关函数
    pub async fn get_all_menu_items(
        pool: &DatabaseConnection,
    ) -> Result<Vec<MenuItem>, Box<dyn std::error::Error>> {
        let menu_items = sqlx::query_as::<_, MenuItem>(
            "SELECT * FROM menu_items ORDER BY sort_order ASC, created_at ASC"
        )
        .fetch_all(pool)
        .await?;

        Ok(menu_items)
    }

    pub async fn get_menu_item_by_id(
        pool: &DatabaseConnection,
        menu_id: Uuid,
    ) -> Result<Option<MenuItem>, Box<dyn std::error::Error>> {
        let menu_item = sqlx::query_as::<_, MenuItem>(
            "SELECT * FROM menu_items WHERE id = $1"
        )
        .bind(menu_id)
        .fetch_optional(pool)
        .await?;

        Ok(menu_item)
    }

    pub async fn create_menu_item(
        pool: &DatabaseConnection,
        menu_data: CreateMenuItemRequest,
    ) -> Result<MenuItem, Box<dyn std::error::Error>> {
        let menu_item = sqlx::query_as::<_, MenuItem>(
            r#"
            INSERT INTO menu_items (title, path, icon, parent_id, sort_order, enabled, permissions)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING *
            "#,
        )
        .bind(&menu_data.title)
        .bind(&menu_data.path)
        .bind(&menu_data.icon)
        .bind(&menu_data.parent_id)
        .bind(&menu_data.sort_order)
        .bind(&menu_data.enabled)
        .bind(&menu_data.permissions)
        .fetch_one(pool)
        .await?;

        Ok(menu_item)
    }

    pub async fn update_menu_item(
        pool: &DatabaseConnection,
        menu_id: Uuid,
        menu_data: UpdateMenuItemRequest,
    ) -> Result<MenuItem, Box<dyn std::error::Error>> {
        let current_menu = sqlx::query_as::<_, MenuItem>(
            "SELECT * FROM menu_items WHERE id = $1"
        )
        .bind(menu_id)
        .fetch_one(pool)
        .await?;

        let title = menu_data.title.unwrap_or(current_menu.title);
        let path = menu_data.path.or(current_menu.path);
        let icon = menu_data.icon.or(current_menu.icon);
        let parent_id = menu_data.parent_id.or(current_menu.parent_id);
        let sort_order = menu_data.sort_order.unwrap_or(current_menu.sort_order);
        let enabled = menu_data.enabled.unwrap_or(current_menu.enabled);
        let permissions = menu_data.permissions.unwrap_or(current_menu.permissions);

        let menu_item = sqlx::query_as::<_, MenuItem>(
            r#"
            UPDATE menu_items 
            SET title = $1, path = $2, icon = $3, parent_id = $4, sort_order = $5, enabled = $6, permissions = $7
            WHERE id = $8
            RETURNING *
            "#,
        )
        .bind(&title)
        .bind(&path)
        .bind(&icon)
        .bind(&parent_id)
        .bind(&sort_order)
        .bind(&enabled)
        .bind(&permissions)
        .bind(menu_id)
        .fetch_one(pool)
        .await?;

        Ok(menu_item)
    }

    pub async fn delete_menu_item(
        pool: &DatabaseConnection,
        menu_id: Uuid,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // 检查是否有子菜单
        let child_count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM menu_items WHERE parent_id = $1"
        )
        .bind(menu_id)
        .fetch_one(pool)
        .await?;

        if child_count > 0 {
            return Err("无法删除有子菜单的项目".into());
        }

        sqlx::query("DELETE FROM menu_items WHERE id = $1")
            .bind(menu_id)
            .execute(pool)
            .await?;

        Ok(())
    }

    // 角色管理相关函数
    pub async fn get_all_roles(
        pool: &DatabaseConnection,
    ) -> Result<Vec<Role>, Box<dyn std::error::Error>> {
        let roles = sqlx::query_as::<_, Role>(
            r#"
            SELECT r.*, COUNT(ur.user_id) as user_count
            FROM roles r
            LEFT JOIN user_roles ur ON r.id = ur.role_id
            GROUP BY r.id
            ORDER BY r.created_at ASC
            "#
        )
        .fetch_all(pool)
        .await?;

        Ok(roles)
    }

    pub async fn get_role_by_id(
        pool: &DatabaseConnection,
        role_id: Uuid,
    ) -> Result<Option<Role>, Box<dyn std::error::Error>> {
        let role = sqlx::query_as::<_, Role>(
            r#"
            SELECT r.*, COUNT(ur.user_id) as user_count
            FROM roles r
            LEFT JOIN user_roles ur ON r.id = ur.role_id
            WHERE r.id = $1
            GROUP BY r.id
            "#
        )
        .bind(role_id)
        .fetch_optional(pool)
        .await?;

        Ok(role)
    }

    pub async fn create_role(
        pool: &DatabaseConnection,
        role_data: CreateRoleRequest,
    ) -> Result<Role, Box<dyn std::error::Error>> {
        // 检查角色名是否已存在
        let existing_role = sqlx::query_as::<_, Role>(
            "SELECT * FROM roles WHERE name = $1"
        )
        .bind(&role_data.name)
        .fetch_optional(pool)
        .await?;
        
        if existing_role.is_some() {
            return Err("角色名已存在".into());
        }

        let role = sqlx::query_as::<_, Role>(
            r#"
            INSERT INTO roles (name, description, permissions)
            VALUES ($1, $2, $3)
            RETURNING *, 0 as user_count
            "#,
        )
        .bind(&role_data.name)
        .bind(&role_data.description)
        .bind(&role_data.permissions)
        .fetch_one(pool)
        .await?;

        Ok(role)
    }

    pub async fn update_role(
        pool: &DatabaseConnection,
        role_id: Uuid,
        role_data: UpdateRoleRequest,
    ) -> Result<Role, Box<dyn std::error::Error>> {
        let current_role = sqlx::query_as::<_, Role>(
            "SELECT * FROM roles WHERE id = $1"
        )
        .bind(role_id)
        .fetch_one(pool)
        .await?;

        let name = role_data.name.clone().unwrap_or(current_role.name);
        let description = role_data.description.unwrap_or(current_role.description);
        let permissions = role_data.permissions.unwrap_or(current_role.permissions);

        // 检查角色名是否重复（排除当前角色）
        if let Some(new_name) = role_data.name {
            let existing_role = sqlx::query_as::<_, Role>(
                "SELECT * FROM roles WHERE name = $1 AND id != $2"
            )
            .bind(&new_name)
            .bind(role_id)
            .fetch_optional(pool)
            .await?;
            
            if existing_role.is_some() {
                return Err("角色名已存在".into());
            }
        }

        let role = sqlx::query_as::<_, Role>(
            r#"
            UPDATE roles 
            SET name = $1, description = $2, permissions = $3
            WHERE id = $4
            RETURNING *, 0 as user_count
            "#,
        )
        .bind(&name)
        .bind(&description)
        .bind(&permissions)
        .bind(role_id)
        .fetch_one(pool)
        .await?;

        Ok(role)
    }

    pub async fn delete_role(
        pool: &DatabaseConnection,
        role_id: Uuid,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // 检查是否有用户使用此角色
        let user_count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM user_roles WHERE role_id = $1"
        )
        .bind(role_id)
        .fetch_one(pool)
        .await?;

        if user_count > 0 {
            return Err("无法删除正在使用的角色".into());
        }

        sqlx::query("DELETE FROM roles WHERE id = $1")
            .bind(role_id)
            .execute(pool)
            .await?;

        Ok(())
    }

    pub async fn update_user_role_new(
        pool: &DatabaseConnection,
        user_id: Uuid,
        role_name: String,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // 获取角色ID
        let role = sqlx::query_as::<_, Role>(
            "SELECT * FROM roles WHERE name = $1"
        )
        .bind(&role_name)
        .fetch_optional(pool)
        .await?
        .ok_or("角色不存在")?;

        // 删除用户现有角色
        sqlx::query("DELETE FROM user_roles WHERE user_id = $1")
            .bind(user_id)
            .execute(pool)
            .await?;

        // 添加新角色
        sqlx::query(
            "INSERT INTO user_roles (user_id, role_id) VALUES ($1, $2)"
        )
        .bind(user_id)
        .bind(role.id)
        .execute(pool)
        .await?;

        // 更新用户表中的角色字段
        sqlx::query("UPDATE users SET role = $1 WHERE id = $2")
            .bind(&role_name)
            .bind(user_id)
            .execute(pool)
            .await?;

        Ok(())
    }
}

pub struct PostService;

impl PostService {
    pub async fn create_post(
        pool: &DatabaseConnection,
        post_data: CreatePostRequest,
        author_id: Uuid,
    ) -> Result<Post, Box<dyn std::error::Error>> {
        let status = post_data.status.unwrap_or_else(|| "draft".to_string());
        
        let post = sqlx::query_as::<_, Post>(
            r#"
            INSERT INTO posts (title, content, author_id, status)
            VALUES ($1, $2, $3, $4)
            RETURNING *
            "#,
        )
        .bind(&post_data.title)
        .bind(&post_data.content)
        .bind(author_id)
        .bind(&status)
        .fetch_one(pool)
        .await?;

        Ok(post)
    }

    pub async fn get_posts(pool: &DatabaseConnection) -> Result<Vec<Post>, Box<dyn std::error::Error>> {
        let posts = sqlx::query_as::<_, Post>(
            "SELECT * FROM posts ORDER BY created_at DESC"
        )
        .fetch_all(pool)
        .await?;
        
        Ok(posts)
    }

    pub async fn get_post_by_id(pool: &DatabaseConnection, post_id: Uuid) -> Result<Option<Post>, Box<dyn std::error::Error>> {
        let post = sqlx::query_as::<_, Post>(
            "SELECT * FROM posts WHERE id = $1"
        )
        .bind(post_id)
        .fetch_optional(pool)
        .await?;
        
        Ok(post)
    }

    pub async fn update_post(
        pool: &DatabaseConnection,
        post_id: Uuid,
        post_data: CreatePostRequest,
        author_id: Uuid,
    ) -> Result<Post, Box<dyn std::error::Error>> {
        let status = post_data.status.unwrap_or_else(|| "draft".to_string());
        
        let post = sqlx::query_as::<_, Post>(
            r#"
            UPDATE posts 
            SET title = $1, content = $2, status = $3, updated_at = NOW()
            WHERE id = $4 AND author_id = $5
            RETURNING *
            "#,
        )
        .bind(&post_data.title)
        .bind(&post_data.content)
        .bind(&status)
        .bind(post_id)
        .bind(author_id)
        .fetch_optional(pool)
        .await?
        .ok_or("文章不存在或无权限修改")?;

        Ok(post)
    }

    pub async fn delete_post(
        pool: &DatabaseConnection,
        post_id: Uuid,
        author_id: Uuid,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let result = sqlx::query(
            "DELETE FROM posts WHERE id = $1 AND author_id = $2"
        )
        .bind(post_id)
        .bind(author_id)
        .execute(pool)
        .await?;
        
        if result.rows_affected() == 0 {
            return Err("文章不存在或无权限删除".into());
        }
        
        Ok(())
    }

}
