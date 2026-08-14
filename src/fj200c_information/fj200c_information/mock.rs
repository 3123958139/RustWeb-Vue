//! # 进程内模拟控制器
//!
//! 无硬件时的模拟数据源：后台线程按 20Hz 生成合法 60 字节帧放入通道，
//! `recv()` 从通道取数据。从 fj200c_information.informatization 的 control/mock.rs 移植。
//!
//! ## 实现要点
//!
//! - **停止信号**（`STOP_SIGNAL`）：所有模拟线程每次循环开头检查，
//!   置 true 后最多 50ms 内退出
//! - **通道**：`mpsc::Receiver` 供 `recv()` 通过 `try_recv` 非阻塞轮询 +
//!   短暂睡眠实现阻塞语义
//! - **正弦叠加噪声模拟**：`0.5 + 0.5 * sin(t * freq)` 生成 0~1 之间周期性
//!   变化的基础值，叠加随机噪声，模拟真实传感器数据的缓变趋势
//!   （海拔、转速、温度等字段）
//! - **trait 对象**：`Arc<dyn IoControl>` 把 MockControl 和 ComControl
//!   统一为一个类型，会话代码无需关心底层实现

use crate::common::utils::parse_hex;
use crate::fj200c_information::IoControl;
use rand::Rng;
use std::error::Error;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

/// 全局停止信号（所有模拟线程共享）
///
/// 置为 `true` 后，所有 `MockControl` 和 `mock_feeder` 线程会在 50ms 内退出。
pub static STOP_SIGNAL: AtomicBool = AtomicBool::new(false);

/// 模拟帧类型字节序列（与 decode.rs 的 FrameType 枚举一一对应）
pub const MOCK_FRAME_TYPES: [u8; 8] = [0xEF, 0xED, 0xDE, 0xDC, 0xBD, 0xDB, 0xBF, 0xBE];

/// 帧总长度（字节）
const FRAME_LEN: usize = 0x3C;
/// 帧头标识
const HEADER: [u8; 3] = [0xEB, 0x90, 0x3C];
/// 模拟帧生成间隔（毫秒，20Hz）
const MOCK_INTERVAL_MS: u64 = 50;

/// 构造一帧合法的模拟数据（校验字节自动计算）。
/// 各字段通过正弦函数叠加随机噪声生成，模拟真实传感器数据变化趋势。
pub fn make_mock_frame(seq: u32) -> [u8; FRAME_LEN] {
    let mut rng = rand::thread_rng();
    let mut f = [0u8; FRAME_LEN];
    f[0..3].copy_from_slice(&HEADER);
    f[3] = MOCK_FRAME_TYPES[(seq as usize) % MOCK_FRAME_TYPES.len()];

    let t = seq as f32 * 0.1;

    f[4] = rng.gen_range(0..=255);
    f[5] = 0;
    let hbgd =
        (2000.0 + 8000.0 * (0.5 + 0.5 * (t * 0.1).sin()) + rng.gen_range(-50.0..50.0)) as i16;
    f[6..8].copy_from_slice(&hbgd.to_le_bytes());

    let ng = (5000.0 + 5500.0 * (0.5 + 0.5 * (t * 0.15).sin()) + rng.gen_range(-20.0..20.0)) as i16;
    f[8..10].copy_from_slice(&ng.to_le_bytes());

    let pqwd =
        (5730.0 + 6000.0 * (0.5 + 0.5 * (t * 0.2).sin()) + rng.gen_range(-30.0..30.0)) as i16;
    f[10..12].copy_from_slice(&pqwd.to_le_bytes());

    let jqwd =
        (2730.0 + 500.0 * (0.5 + 0.5 * (t * 0.08).sin()) + rng.gen_range(-10.0..10.0)) as i16;
    f[12..14].copy_from_slice(&jqwd.to_le_bytes());

    let np = (5000.0 + 6000.0 * (0.5 + 0.5 * (t * 0.18).sin()) + rng.gen_range(-30.0..30.0)) as i16;
    f[14..16].copy_from_slice(&np.to_le_bytes());

    let ym = (40.0 + 50.0 * (0.5 + 0.5 * (t * 0.05).sin()) + rng.gen_range(-3.0..3.0)) as i16;
    f[16..18].copy_from_slice(&ym.to_le_bytes());

    let state_cycle = (seq as u64 / 50) % 5;
    f[18] = match state_cycle {
        0 => 0x06,
        1 => 0x06,
        2 => 0x07,
        3 => 0x06,
        _ => 0x05,
    };

    f[19] = (245 + rng.gen_range(-10..10)) as u8;

    f[20] = 0xA2;

    let has_fault = rng.gen_bool(0.05);
    if has_fault {
        f[21] = rng.gen_range(1..50);
        f[22] = 0;
    } else {
        f[21] = 0;
        f[22] = 0;
    }
    f[23] = 0;
    f[24] = 0;

    f[25] = (60 + rng.gen_range(-15..15)) as u8;

    let hywd =
        (3330.0 + 600.0 * (0.5 + 0.5 * (t * 0.12).sin()) + rng.gen_range(-20.0..20.0)) as i16;
    f[26..28].copy_from_slice(&hywd.to_le_bytes());

    let ryyl = (150 + rng.gen_range(-30..30)) as i16;
    f[28..30].copy_from_slice(&ryyl.to_le_bytes());

    let dlwls_a =
        (5000.0 + 5500.0 * (0.5 + 0.5 * (t * 0.16).sin()) + rng.gen_range(-40.0..40.0)) as i16;
    f[32..34].copy_from_slice(&dlwls_a.to_le_bytes());

    let dlwls_b =
        (4950.0 + 5500.0 * (0.5 + 0.5 * (t * 0.16).sin()) + rng.gen_range(-40.0..40.0)) as i16;
    f[34..36].copy_from_slice(&dlwls_b.to_le_bytes());

    let hrqck =
        (3230.0 + 400.0 * (0.5 + 0.5 * (t * 0.1).sin()) + rng.gen_range(-15.0..15.0)) as i16;
    f[37..39].copy_from_slice(&hrqck.to_le_bytes());

    f[39] = 0x01;

    let fingerprint = 0xABCD1234u32;
    f[45..49].copy_from_slice(&fingerprint.to_le_bytes());

    let sum: u16 = f[..FRAME_LEN - 1].iter().map(|&b| b as u16).sum::<u16>() % 256;
    f[FRAME_LEN - 1] = sum as u8;
    f
}

/// 进程内模拟数据控制器
///
/// 内部启动一个后台线程按 20Hz 生成模拟帧，通过 `mpsc` 通道传递给 `recv()`。
/// 实现 `IoControl` trait，可与 `ComControl` 互换使用。
pub struct MockControl {
    /// 通道接收端（`Mutex` 保护，因为 `recv` 需要 `&self` 但内部需要可变访问）
    rx: Mutex<Receiver<Vec<u8>>>,
}

impl MockControl {
    /// 创建模拟控制器并启动后台帧生成线程
    ///
    /// 后台线程循环调用 `make_mock_frame` 生成模拟帧，
    /// 通过 `mpsc::Sender` 发送到通道，`recv()` 方法从通道接收。
    /// 创建模拟数据源，启动后台帧生成线程
    ///
    /// 线程以 `MOCK_INTERVAL_MS` 间隔持续生成模拟帧，
    /// 全局停止信号（`STOP_SIGNAL`）置位后自动退出。
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel::<Vec<u8>>();

        // 启动后台帧生成线程（std::thread，非 tokio）
        thread::spawn(move || {
            let mut seq: u32 = 0;
            while !STOP_SIGNAL.load(Ordering::Relaxed) {
                let frame = make_mock_frame(seq);
                let _ = tx.send(frame.to_vec());
                seq = seq.wrapping_add(1); // wrapping_add 防止 u32 溢出
                thread::sleep(Duration::from_millis(MOCK_INTERVAL_MS));
            }
        });

        Self { rx: Mutex::new(rx) }
    }
}

impl Default for MockControl {
    fn default() -> Self {
        Self::new()
    }
}

/// `IoControl` trait 的实现：将 `recv` 委托给 `mpsc` 通道接收
///
/// - `send`：模拟模式下忽略发送数据（返回 0）
/// - `recv`：从通道非阻塞轮询 + 5ms 睡眠模拟阻塞语义
impl IoControl for MockControl {
    fn send(&self, _buf: &[u8]) -> Result<usize, Box<dyn Error>> {
        // 模拟模式下发送操作为空操作
        Ok(0)
    }

    fn recv(&self, buf: &mut [u8]) -> Result<usize, Box<dyn Error>> {
        loop {
            // 检查全局停止信号
            if STOP_SIGNAL.load(Ordering::Relaxed) {
                return Err("模拟数据源已停止".into());
            }
            let rx = self.rx.lock().unwrap_or_else(|e| e.into_inner());
            match rx.try_recv() {
                Ok(data) => {
                    let n = data.len().min(buf.len());
                    buf[..n].copy_from_slice(&data[..n]);
                    return Ok(n);
                }
                Err(mpsc::TryRecvError::Empty) => {
                    drop(rx); // 释放锁后再睡眠，避免阻塞发送端
                    thread::sleep(Duration::from_millis(5));
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    return Err("模拟数据源已断开".into());
                }
            }
        }
    }
}

/// 把十六进制命令字符串解析为字节数组
///
/// 输入格式如 `"EB 90 64 EF"` 或 `"EB9064EF"`（支持空格分隔和 `0x` 前缀）。
/// 用于前端发送命令到设备。
pub fn parse_command_hex(hex: &str) -> Option<Vec<u8>> {
    parse_hex(&hex.replace(' ', "").replace("0x", ""))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_frame_valid() {
        let frame = make_mock_frame(0);
        let sum: u16 = frame[..FRAME_LEN - 1]
            .iter()
            .map(|&b| b as u16)
            .sum::<u16>()
            % 256;
        assert_eq!(sum as u8, frame[FRAME_LEN - 1]);
        assert_eq!(&frame[..3], &HEADER);
    }

    #[test]
    fn test_mock_control_recv() {
        let control = MockControl::new();
        let mut buf = [0u8; FRAME_LEN];
        let n = control.recv(&mut buf).unwrap();
        assert_eq!(n, FRAME_LEN);
    }
}
