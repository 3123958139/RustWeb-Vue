//! # ftj1c 四槽帧缓冲（主备切换）
//!
//! 复用公共 `crate::common::quad_frame::QuadFrame` 泛型实现，
//! 本模块固定 95 字节帧长（`EB 90 5B` 帧头协议）并 re-export 相关常量。

/// 95 字节帧长的四槽帧缓冲（别名固定泛型参数）
pub type QuadFrame = crate::common::quad_frame::QuadFrame<FRAME_LEN>;

/// 主源帧槽标识
pub const SRC_PRIMARY: u8 = 0;
/// 备用源帧槽标识
pub const SRC_SECONDARY: u8 = 1;

/// 单帧长度（字节数）
pub const FRAME_LEN: usize = 95;
