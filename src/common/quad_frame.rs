//! # 公共四槽帧缓冲（主备切换）
//!
//! 四个独立的数据槽位，支持主/备数据源自动切换：
//! - 主源（`SRC_PRIMARY`）发送数据时记录心跳时间戳
//! - 主源超时（`TIMEOUT_MS` = 1000ms）后自动切换到备源
//! - 备源发送数据时不记录心跳，但可以随时写入（预缓存，切换时无缝接管）
//! - 主源恢复后自动切回
//!
//! 通过 `const FRAME_LEN` 泛型参数适配不同协议帧长
//! （fj200c_information 使用 100 字节帧，ftj1c 使用 95 字节帧）。
//!
//! ## 无锁设计
//!
//! - `ArcSwap`：RCU（Read-Copy-Update）无锁读写
//! - `AtomicU32` + CAS：序号去重（防重复/回滚）
//! - `AtomicU8`：当前活跃源
//! - `AtomicI64`：主源心跳时间戳

use arc_swap::ArcSwap;
use std::sync::atomic::{AtomicI64, AtomicU32, AtomicU8, Ordering};
use std::sync::Arc;

use crate::common::utils::now_ms;

/// 主数据源标识
pub const SRC_PRIMARY: u8 = 0;
/// 备数据源标识
pub const SRC_SECONDARY: u8 = 1;
/// 槽位数量（4 个独立缓冲区）
pub const SLOT_COUNT: usize = 4;
/// 主源超时阈值（毫秒），超过此时间无心跳则切换到备源
pub const TIMEOUT_MS: i64 = 1000;

/// 四槽帧缓冲器
///
/// 每个槽位独立存储一帧数据，支持主备数据源自动切换。
/// 读方无需持锁即可获取任意槽位的数据快照。
pub struct QuadFrame<const FRAME_LEN: usize> {
    /// 4 个帧数据槽位（每个槽位是独立的原子指针）
    slots: [ArcSwap<[u8; FRAME_LEN]>; SLOT_COUNT],
    /// 4 个槽位的序号计数器（用于去重和 CAS 更新）
    seqs: [AtomicU32; SLOT_COUNT],
    /// 当前活跃数据源（主/备）
    active_source: AtomicU8,
    /// 主源最后心跳时间戳（毫秒）
    primary_heartbeat: AtomicI64,
}

impl<const FRAME_LEN: usize> QuadFrame<FRAME_LEN> {
    /// 创建新的四槽帧缓冲器（所有槽位初始化为零字节）
    pub fn new() -> Self {
        Self {
            slots: [
                ArcSwap::new(Arc::new([0u8; FRAME_LEN])),
                ArcSwap::new(Arc::new([0u8; FRAME_LEN])),
                ArcSwap::new(Arc::new([0u8; FRAME_LEN])),
                ArcSwap::new(Arc::new([0u8; FRAME_LEN])),
            ],
            seqs: [
                AtomicU32::new(0),
                AtomicU32::new(0),
                AtomicU32::new(0),
                AtomicU32::new(0),
            ],
            active_source: AtomicU8::new(SRC_PRIMARY),
            primary_heartbeat: AtomicI64::new(now_ms()),
        }
    }

    /// 尝试更新指定槽位的帧数据
    ///
    /// 使用 CAS（Compare-And-Swap）原子操作更新序号，保证只有更新的帧才能写入。
    /// 主源写入时更新心跳时间戳；备源写入时检查主源是否超时，超时则切换到备源。
    ///
    /// 返回 `true` 表示更新成功（包括备源在主源超时后接管的情况）。
    pub fn try_update(&self, slot: usize, src: u8, seq: u32, frame: &[u8; FRAME_LEN]) -> bool {
        // CAS 尝试更新序号：只有新序号大于当前序号时才成功
        let prev = self.seqs[slot].load(Ordering::Acquire);
        let seq = seq + 1;
        if seq <= prev {
            return false; // 旧数据，拒绝更新
        }
        if self.seqs[slot]
            .compare_exchange(prev, seq, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return false; // 并发竞争失败
        }

        // 序号更新成功，写入帧数据
        self.slots[slot].store(Arc::new(*frame));

        let now = now_ms();

        if src == SRC_PRIMARY {
            // 主源写入：更新心跳时间戳
            self.primary_heartbeat.store(now, Ordering::Release);
            // 如果当前是备源活跃，切回主源
            if self.active_source.load(Ordering::Acquire) == SRC_SECONDARY {
                self.active_source.store(SRC_PRIMARY, Ordering::Release);
            }
            return true;
        }

        // 备源写入：如果已经是备源活跃，直接返回
        if self.active_source.load(Ordering::Acquire) == SRC_SECONDARY {
            return true;
        }

        // 主源仍活跃但已超时，切换到备源
        let hb = self.primary_heartbeat.load(Ordering::Acquire);
        if now.saturating_sub(hb) > TIMEOUT_MS {
            self.active_source.store(SRC_SECONDARY, Ordering::Release);
            return true;
        }

        false
    }

    /// 读取指定槽位的帧数据（无锁读取）
    pub fn read_slot(&self, slot: usize) -> [u8; FRAME_LEN] {
        *self.slots[slot].load().as_ref()
    }
}

impl<const FRAME_LEN: usize> Default for QuadFrame<FRAME_LEN> {
    fn default() -> Self {
        Self::new()
    }
}
