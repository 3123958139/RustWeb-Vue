//! # 串口控制
//!
//! 对 `serial2` 库的封装（打开/超时复用 `common::serial`），实现 `IoControl` trait。
//! 从 dch crate（fj200c_information.informatization control/com.rs）移植，
//! 构造参数适配 Web 版 config-fj200c_information.ini 的 `[ConnectionN]` 节。
//!
//! ## 设计要点
//!
//! - **serial2（overlapped IO）**：Windows 下读/写请求各自独立挂起，互不串行化。
//!   `read()/write()` 只取 `&self`，会话线程与 mock feeder 线程可无锁并发。
//!   原先 serialport 4.x 是同步 IO，阻塞读会串行化写请求（serialport-rs
//!   issue #29/#124 的已知行为），feeder 50ms 周期发送会被 200ms 读超时
//!   卡成 50~250ms 抖动。
//! - **流控固定关闭**（`FlowControl::None`），防止 XOFF/XON 挂起写入。
//! - **`Drop`**：serial2 句柄析构时自动关闭串口，防止程序退出后串口仍被占用。
//! - **`IoControl` trait 实现**：将串口操作统一为 `send`/`recv` 接口，
//!   与 `MockControl` 共享同一套会话逻辑。

use crate::fj200c_information::IoControl;
use std::error::Error;
use tracing::{debug, error, info};

/// 串口控制器
///
/// 封装串口的打开、读写、关闭操作，实现 `IoControl` trait。
/// serial2 的 `read()/write()` 均取 `&self`，无需互斥锁即可多线程并发。
pub struct ComControl {
    /// serial2 串口句柄（overlapped IO，读写并发安全）
    port: serial2::SerialPort,
    /// 所属连接的配置节名（如 "Connection0"），用于日志标识
    section: String,
}

impl ComControl {
    /// 从 Web 版配置参数创建串口控制器
    ///
    /// 将配置文件中的数字参数映射后打开串口（公共封装 `common::serial`）。
    /// - `parity`: 0=None, 1=Odd, 2=Even
    /// - `stop_bits`: 1=One, 2=Two
    /// - `flow_control`: 兼容旧配置项，固定忽略（始终无流控）
    pub fn new(
        port_name: &str,
        baud_rate: u32,
        data_bits: u8,
        stop_bits: u8,
        parity: u8,
        _flow_control: bool,
        section: &str,
    ) -> Result<Self, Box<dyn Error>> {
        let port = crate::common::serial::open_port(
            port_name,
            baud_rate,
            data_bits,
            stop_bits,
            parity,
            100,
            section,
        )?;
        Ok(Self {
            port,
            section: section.to_string(),
        })
    }

    /// 关闭串口（serial2 句柄随 Drop 自动释放）
    pub fn destroy(&self) {
        info!("[{}] 串口已关闭", self.section);
    }

    /// 向串口发送数据
    ///
    /// 返回实际写入的字节数。
    pub fn send(&self, buf: &[u8]) -> Result<usize, Box<dyn Error>> {
        let n = self.port.write(buf).map_err(|e| {
            let msg = format!("[{}] 串口发送失败: {}", self.section, e);
            error!("{}", msg);
            msg
        })?;
        Ok(n)
    }

    /// 从串口接收数据到缓冲区
    ///
    /// 返回实际读取的字节数。超时错误视为正常轮询间隔（不算致命错误）。
    pub fn recv(&self, buf: &mut [u8]) -> Result<usize, Box<dyn Error>> {
        let n = self.port.read(buf).map_err(|e| {
            debug!("[{}] 串口接收超时: {}", self.section, e);
            Box::new(e) as Box<dyn Error>
        })?;
        Ok(n)
    }

    /// 设置串口接收超时（毫秒）
    ///
    /// 走公共封装 `common::serial::set_read_timeout_shared`，
    /// 只取 `&self`，运行期无锁调整（Windows 下改 COMMTIMEOUTS）。
    pub fn set_timeout(&self, timeout_ms: u64) -> Result<(), Box<dyn Error>> {
        crate::common::serial::set_read_timeout_shared(&self.port, timeout_ms)
    }
}

/// `IoControl` trait 的实现：将串口操作委托给 `ComControl` 的同名方法
///
/// 这种委托模式使得 `ComControl` 可以作为 trait 对象使用，
/// 与 `MockControl` 统一在同一套接口下。
impl IoControl for ComControl {
    fn send(&self, buf: &[u8]) -> Result<usize, Box<dyn Error>> {
        self.send(buf)
    }

    fn recv(&self, buf: &mut [u8]) -> Result<usize, Box<dyn Error>> {
        self.recv(buf)
    }

    fn set_timeout(&self, timeout_ms: u64) -> Result<(), Box<dyn Error>> {
        self.set_timeout(timeout_ms)
    }
}