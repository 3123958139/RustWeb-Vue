//! # 通信协议生成器模块（protocol_generator）
//!
//! 从 demo-protocol（Tauri v2 桌面应用）迁移的通信协议表生成工具，
//! 适配 RustWeb-Vue 的 Axum + Vue3 Web 架构。
//!
//! ## 业务能力
//!
//! - **协议表编辑**：C# 数据类型大小自动重排序号与字节范围（前端 `recalcFields`）
//! - **默认参数表**：服务器侧 `parameters.csv`（UTF-8 BOM + 种子内容）读写，
//!   替代原 Tauri `app_data_dir` 方案
//! - **Markdown 导出**：生成协议表 Markdown 文本（前端弹窗预览/复制）
//! - **Excel 导出**：`rust_xlsxwriter` 生成 xlsx 二进制（前端 Blob 下载），
//!   替代原原生保存对话框
//! - **CSV 解析/序列化**：上传文件内容解析 + 生成带 BOM 的 CSV 文本，
//!   替代前端直接读写本地路径
//! - **打印报表**：前端保留 vue-plugin-hiprint（`window` 环境可用）
//!
//! ## 架构映射（Tauri → Web）
//!
//! | Tauri 概念 | Web 替代 |
//! |---|---|
//! | `invoke("save_protocol"/"load_protocol")` 原生对话框 | 前端 Blob 下载 / 文件上传解析 |
//! | `invoke("export_excel")` 落盘 | `POST /api/protocol_generator/excel` 返回二进制流 |
//! | `default_csv_path(app_data_dir)` | 服务器运行目录 `parameters.csv` |
//! | `open_csv_editor` 多窗口 + `csv-updated` 事件 | 同应用路由页，同页刷新 |
//!
//! ## 模块结构（两级目录约定）
//!
//! | 模块 | 位置 | 用途 |
//! |---|---|---|
//! | `handlers` | 一级 | HTTP 端点 + WebSocket 推送 |
//! | `routes` | 一级 | 子路由定义 |
//! | `services` | 一级 | 业务编排（转发给二级 generator） |
//! | `models` | 二级 | DTO（ProtocolField / ParameterDef，orval 生成前端类型） |
//! | `generator` | 二级 | 核心逻辑（Markdown / Excel / CSV 读写） |

pub mod handlers;
pub mod protocol_generator;
pub mod routes;
pub mod services;

// 再导出二级目录子模块，保持 `crate::protocol_generator::x` 路径不变
pub use protocol_generator::models;