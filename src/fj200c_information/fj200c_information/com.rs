//! # 串口控制
//!
//! 对 `serialport` 4 库的封装，实现 `IoControl` trait。
//! 从 dch crate（fj200c_information.informatization control/com.rs）移植，
//! 构造参数适配 Web 版 config-fj200c_information.ini 的 `[ConnectionN]` 节。
//!
//! ## 关键语法
//!
//! - **`Mutex<Option<Box<dyn SerialPort>>>`**：串口句柄的线程安全存储。
//!   - `Mutex` — 互斥访问，保证同一时刻只有一个线程操作串口
//!   - `Option` — 可能未打开（未创建 / destroy 后）
//!   - `Box<dyn SerialPort>` — trait 对象，支持不同平台后端（Windows/Linux/Mac）
//! - **构建器模式**：`serialport::new(...).data_bits(...).parity(...).stop_bits(...)`
//!   链式调用设置参数，最后 `.open()` 实际打开端口。
//! - **`Drop`**：析构时自动关闭串口，防止程序退出后串口仍被占用。
//! - **`IoControl` trait 实现**：将串口操作统一为 `send`/`recv` 接口，
//!   与 `MockControl` 共享同一套会话逻辑。

use crate::fj200c_information::IoControl;
use serialport::SerialPort;
use std::error::Error;
use std::sync::Mutex;
use tracing::{debug, error, info};

/// 串口校验位类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComParity {
    /// 无校验
    None,
    /// 偶校验
    Even,
    /// 奇校验
    Odd,
}

/// 串口停止位类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComStopBits {
    /// 1 个停止位
    One,
    /// 2 个停止位
    Two,
}

/// 串口配置参数
#[derive(Debug, Clone)]
pub struct ComConfig {
    /// 串口名称（如 "COM1"、"/dev/ttyUSB0"）
    pub port_name: String,
    /// 波特率（如 9600、115200）
    pub baud_rate: u32,
    /// 数据位（5/6/7/8）
    pub data_bits: u8,
    /// 校验位
    pub parity: ComParity,
    /// 停止位
    pub stop_bits: ComStopBits,
    /// 接收超时（毫秒）
    pub timeout_ms: u64,
    /// 是否启用软件流控
    pub flow_control: bool,
}

impl Default for ComConfig {
    fn default() -> Self {
        Self {
            port_name: String::new(),
            baud_rate: 115200,
            data_bits: 8,
            parity: ComParity::None,
            stop_bits: ComStopBits::One,
            timeout_ms: 100,
            flow_control: false,
        }
    }
}

/// 串口控制器
///
/// 封装串口的打开、读写、关闭操作，实现 `IoControl` trait。
/// 内部用 `Mutex` 保护串口句柄，支持多线程安全访问。
pub struct ComControl {
    /// 串口句柄（`None` 表示已关闭）
    port: Mutex<Option<Box<dyn SerialPort>>>,
    /// 串口配置参数
    config: ComConfig,
    /// 所属连接的配置节名（如 "Connection0"），用于日志标识
    section: String,
}

impl ComControl {
    /// 从 Web 版配置参数创建串口控制器
    ///
    /// 将配置文件中的数字参数映射为枚举类型，然后调用 `open_port` 打开串口。
    /// - `parity`: 0=None, 1=Odd, 2=Even
    /// - `stop_bits`: 1=One, 2=Two
    pub fn new(
        port_name: &str,
        baud_rate: u32,
        data_bits: u8,
        stop_bits: u8,
        parity: u8,
        flow_control: bool,
        section: &str,
    ) -> Result<Self, Box<dyn Error>> {
        let config = ComConfig {
            port_name: port_name.to_string(),
            baud_rate,
            data_bits,
            parity: match parity {
                1 => ComParity::Odd,
                2 => ComParity::Even,
                _ => ComParity::None,
            },
            stop_bits: match stop_bits {
                2 => ComStopBits::Two,
                _ => ComStopBits::One,
            },
            timeout_ms: 100,
            flow_control,
        };

        let control = Self {
            port: Mutex::new(None),
            config,
            section: section.to_string(),
        };
        control.open_port()?;
        Ok(control)
    }

    /// 打开串口（使用 serialport 构建器模式配置参数）
    ///
    /// 构建器链式设置数据位、校验位、停止位、超时和流控，
    /// 最后调用 `.open()` 实际打开端口。
    fn open_port(&self) -> Result<(), Box<dyn Error>> {
        let mut builder = serialport::new(&self.config.port_name, self.config.baud_rate);
        builder = builder
            .data_bits(match self.config.data_bits {
                5 => serialport::DataBits::Five,
                6 => serialport::DataBits::Six,
                7 => serialport::DataBits::Seven,
                _ => serialport::DataBits::Eight,
            })
            .parity(match self.config.parity {
                ComParity::None => serialport::Parity::None,
                ComParity::Even => serialport::Parity::Even,
                ComParity::Odd => serialport::Parity::Odd,
            })
            .stop_bits(match self.config.stop_bits {
                ComStopBits::One => serialport::StopBits::One,
                ComStopBits::Two => serialport::StopBits::Two,
            })
            .timeout(std::time::Duration::from_millis(self.config.timeout_ms))
            .flow_control(if self.config.flow_control {
                serialport::FlowControl::Software
            } else {
                serialport::FlowControl::None
            });

        let port = builder.open().map_err(|e| {
            let msg = format!(
                "[{}] 打开串口 {} 失败: {}",
                self.section, self.config.port_name, e
            );
            error!("{}", msg);
            msg
        })?;

        {
            let mut p = self.port.lock().unwrap_or_else(|e| e.into_inner());
            *p = Some(port);
        }

        info!(
            "[{}] 串口 {} 已打开: {} baud, {}{}{}",
            self.section,
            self.config.port_name,
            self.config.baud_rate,
            self.config.data_bits,
            match self.config.parity {
                ComParity::None => "N",
                ComParity::Even => "E",
                ComParity::Odd => "O",
            },
            match self.config.stop_bits {
                ComStopBits::One => "1",
                ComStopBits::Two => "2",
            },
        );
        debug!(
            "[{}] 串口配置: {:?}, 超时: {}ms",
            self.section, self.config, self.config.timeout_ms
        );

        Ok(())
    }

    /// 关闭串口（句柄置 None，资源由 `SerialPort` 的 `Drop` 自动释放）
    pub fn destroy(&self) {
        let mut p = self.port.lock().unwrap_or_else(|e| e.into_inner());
        if p.take().is_some() {
            info!(
                "[{}] 串口 {} 已关闭",
                self.section, self.config.port_name
            );
        }
    }

    /// 向串口发送数据
    ///
    /// 返回实际写入的字节数。串口未打开时返回错误。
    pub fn send(&self, buf: &[u8]) -> Result<usize, Box<dyn Error>> {
        let mut p = self.port.lock().unwrap_or_else(|e| e.into_inner());
        let port = p.as_mut().ok_or_else(|| {
            let msg = format!("[{}] 串口未打开，无法发送", self.section);
            error!("{}", msg);
            msg
        })?;
        let n = port.write(buf).map_err(|e| {
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
        let mut p = self.port.lock().unwrap_or_else(|e| e.into_inner());
        let port = p.as_mut().ok_or_else(|| {
            let msg = format!("[{}] 串口未打开，无法接收", self.section);
            error!("{}", msg);
            msg
        })?;
        let n = port.read(buf).map_err(|e| {
            debug!("[{}] 串口接收超时: {}", self.section, e);
            Box::new(e) as Box<dyn Error>
        })?;
        Ok(n)
    }

    /// 设置串口接收超时（毫秒）
    pub fn set_timeout(&self, timeout_ms: u64) -> Result<(), Box<dyn Error>> {
        let mut p = self.port.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(ref mut port) = *p {
            port.set_timeout(std::time::Duration::from_millis(timeout_ms))
                .map_err(|e| Box::new(e) as Box<dyn Error>)?;
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

/// 析构时自动关闭串口，防止程序退出后串口仍被占用
///
/// `Drop::drop` 是 Rust 的 RAII 机制，对象离开作用域时自动调用。
impl Drop for ComControl {
    fn drop(&mut self) {
        self.destroy();
    }
}
