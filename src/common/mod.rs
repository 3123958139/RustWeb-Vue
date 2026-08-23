//! # 公共模块
//!
//! 本模块包含所有角色共用的基础设施：
//!
//! - `auth`: 认证模块（登录、用户信息、用户管理）
//! - `error`: 错误处理模块（统一错误类型）
//! - `jwt`: JWT 令牌模块（创建、验证）
//! - `middleware`: 中间件模块（认证、权限检查）
//! - `models`: 数据模型模块（用户、权限、请求/响应类型）
//!
//! # 架构说明
//!
//! ```text
//! 请求 → 中间件（认证/权限） → 处理器 → 服务 → 数据库
//! ```
//!
//! 本模块提供：
//! - **认证机制**: JWT 令牌 + 中间件验证
//! - **权限控制**: 基于角色的访问控制（RBAC）
//! - **错误处理**: 统一的错误类型和响应格式
//! - **数据模型**: 用户、权限、请求/响应结构体

// ============ 子模块声明 ============
pub mod auth;           // 认证模块：登录、用户信息、用户管理
pub mod config;         // INI 配置封装（configparser）
pub mod crypto;         // 可逆加密：用户名/邮箱/角色 AES-256-GCM + HMAC 指纹
pub mod csv_writer;     // CSV 写入器（500ms 批量刷新 + Drop flush）
pub mod dto;            // 公共响应 DTO：fj200c_information 共用载荷
pub mod error;          // 错误处理：统一错误类型
pub mod frame_extractor; // 帧提取器（字节流 → 固定长度帧）
pub mod global_var;     // 全局 KV 存储（OnceLock + RwLock）
pub mod io;             // 统一硬件 IO 抽象 trait（IoControl）
pub mod jwt;            // JWT 令牌：创建和验证
pub mod latest_frame;   // 最新帧跟踪器（ArcSwap + CAS 序号去重）
pub mod least_squares;  // 最小二乘法多项式拟合（Gauss 消元）
pub mod middleware;     // 中间件：认证、权限检查
pub mod models;         // 数据模型：用户、权限、请求/响应
pub mod quad_frame;     // 四槽帧缓冲（主备切换，泛型帧长）
pub mod rate_limit;     // 登录速率限制（内存滑动窗口）
pub mod serial;         // serial2 串口公共封装（数字参数映射 / 超时 / 日志）
pub mod service;        // 服务运行时（线程句柄 / 停止标志）
pub mod utils;          // 公共工具函数（hex/ASCII/时间戳/CSV/数字提取）
pub mod ws;             // 公共 WebSocket 事件桥

// 引入 Axum 的 HTTP 状态码
use axum::http::StatusCode;

/// 健康检查端点
///
/// # 功能
/// 返回 HTTP 200 OK，用于：
/// - 负载均衡器检测服务是否正常
/// - 监控系统检查服务可用性
/// - 容器编排（如 Docker、Kubernetes）的健康检查
///
/// # 端点
/// `GET /health`
///
/// # 返回
/// - `StatusCode::OK` (200) - 服务正常
///
/// # 语法说明
///
/// `async fn` 定义异步函数
/// Axum 的处理器（handler）必须是异步函数
pub async fn health_check() -> StatusCode {
    StatusCode::OK
}
