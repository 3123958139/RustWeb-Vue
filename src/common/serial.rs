//! # serial2 串口公共封装
//!
//! fj200c_main / fj200c_information / ftj1c 三处串口打开逻辑的公共部分：
//! 把 Web 配置的数字参数（`data_bits` 5~8、`parity` 0/1/2、`stop_bits` 1/2）
//! 映射为 serial2 枚举，统一 `set_raw()` + 无流控 + 读超时，并输出打开日志。
//!
//! serial2 在 Windows 上用 overlapped IO，`read()/write()` 只取 `&self`，
//! 收发线程可无锁并发，不会出现 serialport 4.x 同步 IO 的
//! "阻塞读串行化写"问题（serialport-rs issue #29/#124）。
//! 跨平台支持 Windows / Linux（Ubuntu）/ macOS，无需改动。

use std::error::Error;
use std::time::Duration;
use tracing::{error, info};

/// 按 Web 配置数字参数打开串口
///
/// # 参数
/// - `port_name`: 串口名（如 "COM1"、"/dev/ttyUSB0"）
/// - `baud_rate`: 波特率（如 9600、115200）
/// - `data_bits`: 数据位（5/6/7/8，其余按 8 处理）
/// - `stop_bits`: 停止位（2 为 Two，其余为 One）
/// - `parity`: 校验位（0=None, 1=Odd, 2=Even，其余为 None）
/// - `read_timeout_ms`: 读超时（毫秒）
/// - `section`: 配置节名，用于日志标识
///
/// # 返回值
/// `Ok(SerialPort)` 已配置并打开的串口；失败返回错误信息字符串。
pub fn open_port(
    port_name: &str,
    baud_rate: u32,
    data_bits: u8,
    stop_bits: u8,
    parity: u8,
    read_timeout_ms: u64,
    section: &str,
) -> Result<serial2::SerialPort, String> {
    let mut port = serial2::SerialPort::open(port_name, |mut settings: serial2::Settings| {
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

    port.set_read_timeout(Duration::from_millis(read_timeout_ms))
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

    Ok(port)
}

/// 运行期调整读超时（毫秒），只取 `&self`
///
/// 用于 `IoControl::set_timeout(&self, ...)` 这类只持不可变引用的场景：
/// Windows 下直接改 COMMTIMEOUTS（ReadIntervalTimeout=MAX 保持
/// "有数据即返回"语义，常数项取目标超时）；非 Windows 平台无法在
/// `&self` 下修改，返回 `Unsupported` 错误。
pub fn set_read_timeout_shared(
    port: &serial2::SerialPort,
    timeout_ms: u64,
) -> Result<(), Box<dyn Error>> {
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
        port.set_windows_timeouts(&timeouts)
            .map_err(|e| Box::new(e) as Box<dyn Error>)
    }
    #[cfg(not(windows))]
    {
        let _ = (port, timeout_ms);
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "运行期读超时调整仅支持 Windows",
        )
        .into())
    }
}
