use crate::models::{User, Post, CreateUserRequest, LoginRequest, CreatePostRequest};
use crate::database::DatabaseConnection;
use bcrypt::{hash, verify, DEFAULT_COST};
use uuid::Uuid;
use chrono::Utc;

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

    pub async fn get_user_by_id(pool: &DatabaseConnection, user_id: Uuid) -> Result<Option<User>, Box<dyn std::error::Error>> {
        let user = sqlx::query_as::<_, User>(
            "SELECT * FROM users WHERE id = $1"
        )
        .bind(user_id)
        .fetch_optional(pool)
        .await?;
        
        Ok(user)
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
