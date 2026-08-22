//! mario 角色模块数据模型：游戏成绩 DTO。

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// 提交成绩请求体（POST /api/mario/scores）
#[derive(Debug, Deserialize, ToSchema)]
pub struct SubmitScoreRequest {
    /// 本局得分
    pub score: i64,
    /// 到达关卡（从 1 开始）
    pub level: i64,
    /// 收集金币数
    pub coins: i64,
    /// 通关耗时（毫秒，0 表示未完成）
    pub time_ms: i64,
}

/// 榜单条目（GET /api/mario/scores 返回）
#[derive(Debug, Serialize, ToSchema)]
pub struct MarioScore {
    /// 排名（1 起始）
    pub ranking: i64,
    /// 提交者用户名
    pub username: String,
    /// 得分
    pub score: i64,
    /// 到达关卡
    pub level: i64,
    /// 收集金币数
    pub coins: i64,
    /// 通关耗时（毫秒，0 表示未完成）
    pub time_ms: i64,
    /// 提交时间
    pub created_at: String,
}

/// 高分榜（分页携带总数，供前端展示「共 N 局」）
#[derive(Debug, Serialize, ToSchema)]
pub struct ScoreList {
    /// 榜单条目
    pub items: Vec<MarioScore>,
    /// 历史总记录数
    pub total: i64,
}

/// 全局统计（GET /api/mario/stats 返回）
#[derive(Debug, Serialize, ToSchema)]
pub struct MarioStats {
    /// 累计游戏总局数
    pub total_games: i64,
    /// 累计收集金币数
    pub total_coins: i64,
    /// 历史最高分
    pub top_score: i64,
    /// 最高分获得者用户名
    pub top_username: Option<String>,
    /// 参与玩家数（不同用户名去重）
    pub players: i64,
}