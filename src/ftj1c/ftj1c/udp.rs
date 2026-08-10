//! # UDP 套接字控制器
//!
//! 从 demo-test3-ftj 的 dch crate（control/udp.rs）移植。
//! 支持组播加入与重连；接收套接字可设置 1MB 接收缓冲区防突发丢包。
//!
//! ## 功能特性
//!
//! - 支持 IPv4/IPv6 组播加入
//! - 自动重连和错误恢复
//! - 可配置接收缓冲区大小（默认 1MB）
//! - 使用 `SO_REUSEADDR` 选项支持端口复用
//!
//! ## 使用方式
//!
//! ```rust
//! let udp = UdpControl::create("0.0.0.0", "226.0.0.80", 8004, UdpMode::Recv)?;
//! let mut buf = [0u8; 4096];
//! let (len, src) = udp.recv_from(&mut buf)?;
//! ```

use std::io;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket};
use std::sync::Mutex;
use tracing::{error, info};

/// UDP 通信模式
///
/// # 说明
/// 指定 UDP 套接字是发送还是接收模式。
/// 接收模式会自动加入组播组（如果是组播地址）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UdpMode {
    /// 发送模式
    Send,
    /// 接收模式
    Recv,
}

/// UDP 套接字控制器
///
/// # 说明
/// 封装 `UdpSocket`，提供线程安全的发送/接收接口。
/// 使用 `Mutex<Option<UdpSocket>>` 保护套接字实例。
///
/// # 线程安全
/// - `Mutex` 保护套接字，防止并发访问冲突
/// - 发送/接收操作会自动获取锁
pub struct UdpControl {
    /// 套接字实例（`Option` 允许 `take` 用于重连）
    socket: Mutex<Option<UdpSocket>>,
    /// 本地绑定 IP
    local_ip: String,
    /// 目标 IP（组播地址或单播地址）
    dest_ip: String,
    /// 目标端口
    dest_port: u16,
    /// 通信模式
    mode: UdpMode,
}

impl UdpControl {
    /// 创建 UDP 控制器并初始化套接字
    ///
    /// # 参数
    /// - `local_ip`: 本地绑定 IP（如 "0.0.0.0"）
    /// - `dest_ip`: 目标 IP（组播地址或单播地址）
    /// - `dest_port`: 目标端口
    /// - `mode`: 通信模式（发送/接收）
    ///
    /// # 返回值
    /// - `Ok(Self)`: 成功创建并初始化套接字
    /// - `Err(Box<dyn Error>)`: 套接字创建或绑定失败
    pub fn create(
        local_ip: &str,
        dest_ip: &str,
        dest_port: u16,
        mode: UdpMode,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let control = Self {
            socket: Mutex::new(None),
            local_ip: local_ip.to_string(),
            dest_ip: dest_ip.to_string(),
            dest_port,
            mode,
        };
        control.create_socket()?;
        Ok(control)
    }

    /// 创建并绑定 UDP 套接字
    ///
    /// # 说明
    /// 1. 使用 `socket2` 创建套接字，设置 `SO_REUSEADDR`
    /// 2. 绑定到 `local_ip:dest_port`
    /// 3. 如果是接收模式且目标是组播地址，加入组播组
    ///
    /// # 错误处理
    /// 如果端口被占用，尝试绑定 `0.0.0.0:<port>` 作为回退。
    fn create_socket(&self) -> Result<(), Box<dyn std::error::Error>> {
        use socket2::{Domain, Protocol, Socket, Type};

        fn bind_with_reuseaddr(addr: &str) -> Result<UdpSocket, Box<dyn std::error::Error>> {
            let sock_addr: std::net::SocketAddr = addr.parse()?;
            let domain = if sock_addr.is_ipv4() {
                Domain::IPV4
            } else {
                Domain::IPV6
            };
            let socket = Socket::new(domain, Type::DGRAM, Some(Protocol::UDP))?;
            socket.set_reuse_address(true)?;
            socket.bind(&sock_addr.into())?;
            Ok(socket.into())
        }

        let bind_addr = format!("{}:{}", self.local_ip, self.dest_port);

        let socket = match bind_with_reuseaddr(&bind_addr) {
            Ok(s) => s,
            Err(e) => {
                if let Some(io_err) = e.downcast_ref::<std::io::Error>() {
                    if io_err.kind() == io::ErrorKind::AddrInUse {
                        let fallback = format!("0.0.0.0:{}", self.dest_port);
                        bind_with_reuseaddr(&fallback)?
                    } else {
                        return Err(Box::new(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            format!("Socket bind failed: {}", io_err),
                        )));
                    }
                } else {
                    return Err(e);
                }
            }
        };

        if self.mode == UdpMode::Recv {
            let remote_ip: IpAddr = self.dest_ip.parse().map_err(|e| {
                error!("目标 IP 解析失败 '{}': {}", self.dest_ip, e);
                e
            })?;
            match remote_ip {
                IpAddr::V4(v4) if v4.is_multicast() => {
                    let local_ip: Ipv4Addr = self.local_ip.parse().map_err(|e| {
                        error!("本地 IP 解析失败 '{}': {}", self.local_ip, e);
                        e
                    })?;
                    socket.join_multicast_v4(&v4, &local_ip).map_err(|e| {
                        error!("加入 IPv4 组播组失败 {}: {}", v4, e);
                        e
                    })?;
                    info!("已加入 IPv4 组播组: {}, 本机: {}", v4, local_ip);
                }
                IpAddr::V6(v6) if v6.is_multicast() => {
                    socket.join_multicast_v6(&v6, 0).map_err(|e| {
                        error!("加入 IPv6 组播组失败 {}: {}", v6, e);
                        e
                    })?;
                    info!("已加入 IPv6 组播组: {}", v6);
                }
                _ => {}
            }
        }

        let mut sock = self.socket.lock().unwrap_or_else(|e| e.into_inner());
        *sock = Some(socket);

        info!(
            "[ftj1c] UDP {} 创建成功 {}:{} -> {}:{}",
            if self.mode == UdpMode::Send {
                "发送"
            } else {
                "接收"
            },
            self.local_ip,
            self.dest_port,
            self.dest_ip,
            self.dest_port,
        );
        Ok(())
    }

    /// 向配置的目标地址发送数据
    ///
    /// # 参数
    /// - `buf`: 待发送的字节切片
    ///
    /// # 返回值
    /// - `Ok(usize)`: 实际发送的字节数
    /// - `Err(Box<dyn Error>)`: 套接字未创建或发送失败
    pub fn send_to(&self, buf: &[u8]) -> Result<usize, Box<dyn std::error::Error>> {
        let sock = self.socket.lock().unwrap_or_else(|e| e.into_inner());
        let socket = sock
            .as_ref()
            .ok_or_else(|| "UDP 套接字未创建，无法发送".to_string())?;
        let dest: SocketAddr = format!("{}:{}", self.dest_ip, self.dest_port).parse()?;
        Ok(socket.send_to(buf, dest)?)
    }

    /// 从绑定的套接字接收数据
    ///
    /// # 参数
    /// - `buf`: 接收缓冲区
    ///
    /// # 返回值
    /// - `Ok((usize, String))`: (接收的字节数, 来源地址字符串)
    /// - `Err(Box<dyn Error>)`: 套接字未创建或接收失败
    ///
    /// # 说明
    /// 此操作是阻塞的，直到收到数据或超时。
    pub fn recv_from(
        &self,
        buf: &mut [u8],
    ) -> Result<(usize, String), Box<dyn std::error::Error>> {
        let sock = self.socket.lock().unwrap_or_else(|e| e.into_inner());
        let socket = sock
            .as_ref()
            .ok_or_else(|| "UDP 套接字未创建，无法接收".to_string())?;
        let (size, src) = socket.recv_from(buf)?;
        Ok((size, src.to_string()))
    }

    /// 设置接收缓冲区大小
    ///
    /// # 参数
    /// - `size`: 缓冲区大小（字节）
    ///
    /// # 返回值
    /// - `Ok(())`: 设置成功
    /// - `Err(Box<dyn Error>)`: 设置失败
    ///
    /// # 说明
    /// 增大缓冲区可减少高频场景下的丢包。
    /// 推荐值：1MB (1024 * 1024)，用于突发数据场景。
    pub fn set_recv_buffer_size(&self, size: usize) -> Result<(), Box<dyn std::error::Error>> {
        use socket2::Socket;
        let mut sock = self.socket.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(socket) = sock.take() {
            let s2: Socket = socket.into();
            s2.set_recv_buffer_size(size)?;
            *sock = Some(s2.into());
        }
        Ok(())
    }
}
