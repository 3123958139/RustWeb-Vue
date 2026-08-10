//! city3d 角色模块 —— 城市 3D 数字孪生展示。
//!
//! 角色：city3d（城市 3D 展示）
//! - 权限点：`Permission::City3dView`（已在 `src/roles.rs` 注册表授予）
//! - 路由：`/api/city3d`（仅拥有 city3d:view 权限的角色可访问）
//!
//! # API 端点
//!
//! | 方法 | 路径 | 说明 |
//! |------|------|------|
//! | GET | `/api/city3d/buildings` | 建筑列表 |
//! | POST | `/api/city3d/buildings` | 创建建筑 |
//! | PUT | `/api/city3d/buildings/:id` | 更新建筑 |
//! | DELETE | `/api/city3d/buildings/:id` | 删除建筑 |
//! | GET | `/api/city3d/districts` | 区域列表 |
//! | POST | `/api/city3d/districts` | 创建区域 |
//! | PUT | `/api/city3d/districts/:id` | 更新区域 |
//! | DELETE | `/api/city3d/districts/:id` | 删除区域 |
//! | GET | `/api/city3d/events` | 事件列表 |
//! | POST | `/api/city3d/events` | 创建事件 |
//! | DELETE | `/api/city3d/events/:id` | 删除事件 |
//! | GET | `/api/city3d/overview` | 城市概览聚合统计 |

pub mod city3d;
pub mod handlers;
pub mod routes;
pub mod services;

// 再导出二级目录子模块，保持既有 `crate::city3d::x` 路径不变
pub use city3d::models;