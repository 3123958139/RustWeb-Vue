//! # 串口控制
//!
//! 对 `serial2` 库的封装，实现 `IoControl` trait。
//! 从 dch crate（fj200c_information.informatization control/com.rs）移植，
//! 构造参数适配 Web 版 config-fj200c_information.ini 的 `[ConnectionN]` 节。
//!
//! ## 关键点
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
use std::time::Duration;
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
    /// 将配置文件中的数字参数映射为枚举类型，然后打开串口。
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
        let mut port =
            serial2::SerialPort::open(port_name, |mut settings: serial2::Settings| {
                settings.set_raw();
                settings.set_baud_rate(baud_rate)?;
                settings.set_char_size(match data_bits {
                    5 => serial2::CharSize::Bits5,
                    6 => serial2::CharSize::Bits6,
                    7 => serial2::CharSize::Bits7,
                    _ => serial2::CharSize::Bits8,
                });
                settings.set_stop_bits(match stop_bits {
                    2 => serial2::StopBits::Two,
                    _ => serial2::StopBits::One,
                });
                settings.set_parity(match parity {
                    1 => serial2::Parity::Odd,
                    2 => serial2::Parity::Even,
                    _ => serial2::Parity::None,
                });
                settings.set_flow_control(serial2::FlowControl::None);
                Ok(settings)
            })
            .map_err(|e| {
                let msg = format!("[{}] 打开串口 {} 失败: {}", section, port_name, e);
                error!("{}", msg);
                msg
            })?;
        port.set_read_timeout(Duration::from_millis(100))
            .map_err(|e| {
                let msg = format!("[{}] 设置读超时失败: {}", section, e);
                error!("{}", msg);
                msg
            })?;
        info!(
            "[{}] 串口 {} 已打开(overlapped 并发读写, 无流控): {} baud, {}{}{}",
            section,
            port_name,
            baud_rate,
            data_bits,
            match parity {
                1 => "O",
                2 => "E",
                _ => "N",
            },
            match stop_bits {
                2 => "2",
                _ => "1",
            },
        );
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
    /// serial2 的 `set_windows_timeouts` 只取 `&self`，运行期无锁调整。
    pub fn set_timeout(&self, timeout_ms: u64) -> Result<(), Box<dyn Error>> {
        #[cfg(windows)]
        {
            let timeout = timeout_ms.min(u32::MAX as u64 - 1) as u32;
            let timeouts = serial2::os::windows::CommTimeouts {
                read_interval_timeout: u32::MAX,
                read_total_timeout_multiplier: u32::MAX,
                read_total_timeout_constant: timeout,
                write_total_timeout_multiplier: 0,
                write_total_timeout_constant: timeout,
            };
            self.port
                .set_windows_timeouts(&timeouts)
                .map_err(|e| Box::new(e) as Box<dyn Error>)?;
        }
        #[cfg(not(windows))]
        {
            let _ = timeout_ms;
        }
        Ok(())
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