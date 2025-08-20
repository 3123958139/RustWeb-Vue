use sqlx::{PgPool, Row};
use crate::models::{User, Post};

pub type DatabaseConnection = PgPool;

pub async fn init_database(database_url: &str) -> Result<DatabaseConnection, Box<dyn std::error::Error>> {
    println!("正在连接数据库: {}", database_url);
    
    let pool = PgPool::connect(database_url).await?;
    println!("数据库连接成功");
    
    // 创建表
    create_tables(&pool).await?;
    println!("数据库表创建完成");
    
    Ok(pool)
}

pub async fn create_tables(pool: &PgPool) -> Result<(), Box<dyn std::error::Error>> {
    // 创建用户表
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS users (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            username VARCHAR(50) UNIQUE NOT NULL,
            email VARCHAR(100) UNIQUE NOT NULL,
            password_hash VARCHAR(255) NOT NULL,
            role VARCHAR(20) NOT NULL DEFAULT 'user',
            created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
            updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await?;

    // 创建文章表
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS posts (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            title VARCHAR(200) NOT NULL,
            content TEXT NOT NULL,
            author_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            status VARCHAR(20) NOT NULL DEFAULT 'draft',
            created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
            updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await?;

    Ok(())
}
