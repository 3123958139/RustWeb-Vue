//! # 最新帧跟踪器
//!
//! 使用 `ArcSwap` 和 `AtomicU32` 实现无锁的"最新一帧"存储。
//! CAS 序号去重防止旧数据覆盖新数据。

use arc_swap::ArcSwap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

pub struct LatestFrame<const N: usize> {
    slot: ArcSwap<[u8; N]>,
    seq: AtomicU32,
}

impl<const N: usize> LatestFrame<N> {
    pub fn new() -> Self {
        Self {
            slot: ArcSwap::new(Arc::new([0u8; N])),
            seq: AtomicU32::new(0),
        }
    }

    pub fn update(&self, seq: u32, frame: &[u8; N]) -> bool {
        let prev = self.seq.load(Ordering::Acquire);
        let seq = seq.wrapping_add(1);
        if seq <= prev {
            return false;
        }
        if self
            .seq
            .compare_exchange(prev, seq, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return false;
        }
        self.slot.store(Arc::new(*frame));
        true
    }

    pub fn read(&self) -> [u8; N] {
        *self.slot.load().as_ref()
    }

    pub fn read_arc(&self) -> Arc<[u8; N]> {
        self.slot.load_full()
    }

    pub fn seq(&self) -> u32 {
        self.seq.load(Ordering::Acquire)
    }
}

impl<const N: usize> Default for LatestFrame<N> {
    fn default() -> Self {
        Self::new()
    }
}
