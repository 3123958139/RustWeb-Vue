//! # 帧数据复合存储
//!
//! 存储最近一帧的解码字段和原始帧，供会话线程与 WebSocket 快照共享。
//! 从 dch crate（fj200c_information.informatization）移植（原实现存多帧历史，此处简化为最新帧）。
//!
//! ## 并发设计
//!
//! - **`ArcSwap<Vec<String>>`**：字段列表采用原子可交换指针，读方无需持锁
//!   即可拿到稳定快照（`load()` 返回 Guard），写方替换整个指针
//! - **`RwLock<Vec<u8>>`**：原始帧数据读写锁，读多写少

use arc_swap::ArcSwap;
use std::sync::RwLock;

/// 帧数据复合存储：同时保存解码后的字段列表和原始帧字节
pub struct FrameBundle {
    /// 解码后的 28 列字段值（原子指针，无锁读取）
    frames: ArcSwap<Vec<String>>,
    /// 原始帧数据（60 字节，读写锁保护）
    last_frame: RwLock<Vec<u8>>,
}

impl FrameBundle {
    /// 创建新的空 FrameBundle 实例
    pub fn new() -> Self {
        Self {
            frames: ArcSwap::from_pointee(Vec::new()),
            last_frame: RwLock::new(Vec::new()),
        }
    }

    /// 更新最新一帧的数据（解码字段 + 原始帧字节）
    ///
    /// 使用 `ArcSwap::store` 替换整个指针，读方通过 `load()` 获取旧数据的稳定快照。
    pub fn update(&self, decoded: Vec<String>, frame: &[u8]) {
        self.frames.store(std::sync::Arc::new(decoded));
        *self.last_frame.write().unwrap_or_else(|e| e.into_inner()) = frame.to_vec();
    }

    /// 获取最近一帧的解码字段（28 列字符串）
    ///
    /// 返回克隆的 `Vec<String>`，调用方可自由修改不影响共享数据。
    pub fn frames(&self) -> Vec<String> {
        self.frames.load().as_ref().clone()
    }
}

/// 实现 `Default` trait，使 `FrameBundle::default()` 等同于 `FrameBundle::new()`
impl Default for FrameBundle {
    fn default() -> Self {
        Self::new()
    }
}
