//! # 模拟数据发生器（Mock Feeder）
//!
//! 在模拟模式下，把构造的模拟帧通过 `IoControl::send` 注入数据通道，
//! 让会话线程的读取路径与真实串口完全一致。
//! 从 fj200c_information.informatization 的 backend/mock.rs 的 feeder 线程移植。
//!
//! ## 实现要点
//!
//! - `Arc<dyn IoControl>` 使 feeder 可注入任意 IO 实现（真实串口或模拟通道）
//! - feeder 线程循环直到 `STOP_SIGNAL` 置位，`move` 闭包持有 IO 句柄所有权

use crate::fj200c_information::mock::{make_mock_frame, STOP_SIGNAL};
use crate::fj200c_information::IoControl;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

/// 模拟帧注入间隔（毫秒，20Hz）
const FEED_INTERVAL_MS: u64 = 50;

/// 启动一个 feeder 线程，周期性向 IO 设备写入模拟帧
///
/// feeder 线程通过 `control.send()` 写入数据，与真实串口的数据写入路径完全一致。
/// 线程会在 `STOP_SIGNAL` 被设置后自动退出。
pub fn start_mock_feeder(control: Arc<dyn IoControl>) {
    thread::spawn(move || {
        let mut seq: u32 = 0;
        while !STOP_SIGNAL.load(std::sync::atomic::Ordering::Relaxed) {
            let frame = make_mock_frame(seq);
            if control.send(&frame).is_err() {
                break; // IO 设备已关闭，退出 feeder
            }
            seq = seq.wrapping_add(1);
            thread::sleep(Duration::from_millis(FEED_INTERVAL_MS));
        }
    });
}
