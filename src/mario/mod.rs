//! mario 角色模块 —— 超级马里奥复刻游戏。
//!
//! 角色：mario（超级马里奥复刻游戏）
//! - 权限点：`Permission::MarioMonitor`（已在 `src/roles.rs` 注册表授予）
//! - 路由：`/api/mario`（仅拥有 mario:monitor 权限的角色可访问）
//!
//! # 设计说明
//!
//! 本模块的游戏**前端运行**于 `frontend/mario/`（Canvas 平台跳跃玩法，
//! 见 `src/mario/views/GameView.vue`），后端只负责**成绩持久化与排行榜**：
//! 每局游戏结束后前端把 `score / level / coins / time_ms` 提交到
//! `POST /api/mario/scores`，并可从 `GET /api/mario/scores` 拉取高分榜、
//! `GET /api/mario/stats` 获取全局统计。
//!
//! # API 端点
//!
//! | 方法 | 路径 | 说明 |
//! |------|------|------|
//! | GET | `/api/mario/scores?limit=` | 高分榜（按分数倒序） |
//! | POST | `/api/mario/scores` | 提交一局游戏成绩 |
//! | GET | `/api/mario/stats` | 全局统计（总局数 / 总金币 / 最高分） |

pub mod handlers;
pub mod models;
pub mod routes;
pub mod services;