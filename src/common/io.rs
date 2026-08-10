//! # 统一硬件 IO 抽象
//!
//! 串口 / 进程内模拟共用的 trait，通过 trait 对象统一 `ComControl` 和 `MockControl`。

pub trait IoControl: Send + Sync {
    fn send(&self, buf: &[u8]) -> Result<usize, Box<dyn std::error::Error>>;
    fn recv(&self, buf: &mut [u8]) -> Result<usize, Box<dyn std::error::Error>>;
    fn set_timeout(&self, _timeout_ms: u64) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
}
