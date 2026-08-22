//! mario 角色模块 service：游戏成绩数据访问层（SQLite）。

use crate::database::DatabaseConnection;
use crate::mario::models::{MarioScore, MarioStats, ScoreList, SubmitScoreRequest};
use uuid::Uuid;

pub struct MarioService;

impl MarioService {
    /// 高分榜（按分数倒序，条数限定）
    pub async fn list_scores(
        pool: &DatabaseConnection,
        limit: i32,
    ) -> Result<ScoreList, Box<dyn std::error::Error>> {
        let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM mario_scores")
            .fetch_one(pool)
            .await?;

        let rows = sqlx::query_as::<_, (Uuid, String, i64, i64, i64, i64, String)>(
            r#"
            SELECT id, username, score, level, coins, time_ms, created_at
            FROM mario_scores
            ORDER BY score DESC, created_at ASC
            LIMIT $1
            "#,
        )
        .bind(limit)
        .fetch_all(pool)
        .await?;

        let items: Vec<MarioScore> = rows
            .into_iter()
            .enumerate()
            .map(|(i, (_id, username, score, level, coins, time_ms, created_at))| MarioScore {
                ranking: i as i64 + 1,
                username,
                score,
                level,
                coins,
                time_ms,
                created_at,
            })
            .collect();

        Ok(ScoreList { items, total: total.0 })
    }

    /// 提交一局成绩（按 score 计算当前排名）
    pub async fn submit_score(
        pool: &DatabaseConnection,
        username: &str,
        body: SubmitScoreRequest,
    ) -> Result<MarioScore, Box<dyn std::error::Error>> {
        let id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO mario_scores (id, username, score, level, coins, time_ms, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, CURRENT_TIMESTAMP)
            "#,
        )
        .bind(id)
        .bind(username)
        .bind(body.score)
        .bind(body.level)
        .bind(body.coins)
        .bind(body.time_ms)
        .execute(pool)
        .await?;

        // 排名 = 分数高于本局的记录数 + 1
        let higher: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM mario_scores WHERE score > $1")
            .bind(body.score)
            .fetch_one(pool)
            .await?;

        let created_at: (String,) =
            sqlx::query_as("SELECT created_at FROM mario_scores WHERE id = $1")
                .bind(id)
                .fetch_one(pool)
                .await?;

        Ok(MarioScore {
            ranking: higher.0 + 1,
            username: username.to_string(),
            score: body.score,
            level: body.level,
            coins: body.coins,
            time_ms: body.time_ms,
            created_at: created_at.0,
        })
    }

    /// 全局统计
    pub async fn get_stats(pool: &DatabaseConnection) -> Result<MarioStats, Box<dyn std::error::Error>> {
        let total_games: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM mario_scores")
            .fetch_one(pool)
            .await?;
        let total_coins: (Option<i64>,) =
            sqlx::query_as("SELECT SUM(coins) FROM mario_scores").fetch_one(pool).await?;
        let top_score: (Option<i64>,) =
            sqlx::query_as("SELECT MAX(score) FROM mario_scores").fetch_one(pool).await?;
        let players: (i64,) =
            sqlx::query_as("SELECT COUNT(DISTINCT username) FROM mario_scores")
                .fetch_one(pool)
                .await?;
        let top_username: Option<(String,)> = sqlx::query_as(
            "SELECT username FROM mario_scores ORDER BY score DESC LIMIT 1",
        )
        .fetch_optional(pool)
        .await?;

        Ok(MarioStats {
            total_games: total_games.0,
            total_coins: total_coins.0.unwrap_or(0),
            top_score: top_score.0.unwrap_or(0),
            top_username: top_username.map(|t| t.0),
            players: players.0,
        })
    }
}