//! # UDP 链路（地面站 ↔ 飞控）
//!
//! 地面站绑定本地端口（默认 14550）监听飞控遥测，向**学习到的对端地址**
//! （最近一次收包的来源，模拟器 / 真实飞控自动适配）发送命令；
//! 未收到任何包时回退到配置的 `TargetIp:TargetPort`。
//!
//! 使用 socket2 设置 `SO_REUSEADDR`（多个地面站可同端口共存，与 QGC 行为一致）。

use std::io;
use std::net::{SocketAddr, UdpSocket};
use std::sync::Mutex;
use tracing::info;

/// UDP 链路控制器
///
/// - 接收：阻塞 `recv_from`，由调用方设置读超时实现轮询退出
/// - 发送：目标地址 = 学习到的对端（最近收包来源）或配置回退地址
pub struct UdpLink {
    socket: Mutex<UdpSocket>,
    /// 学习到的对端地址（最近一次收包来源）
    learned: Mutex<Option<SocketAddr>>,
    /// 配置回退地址（未收到任何包时使用）
    fallback: SocketAddr,
    local: SocketAddr,
}

impl UdpLink {
    /// 创建 UDP 链路并绑定 `local_ip:local_port`
    ///
    /// 绑定失败（端口被占用）时返回错误信息；`SO_REUSEADDR` 允许与
    /// 其他进程共享端口（模拟器回环发送到同一端口时不受影响）。
    pub fn create(local_ip: &str, local_port: u16, target_ip: &str, target_port: u16) -> Result<Self, String> {
        use socket2::{Domain, Protocol, Socket, Type};
        let bind_addr: SocketAddr = format!("{}:{}", local_ip, local_port)
            .parse()
            .map_err(|e| format!("本地地址非法 {}:{}: {}", local_ip, local_port, e))?;
        let sock = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))
            .map_err(|e| format!("创建 UDP 套接字失败: {}", e))?;
        sock.set_reuse_address(true)
            .map_err(|e| format!("设置 SO_REUSEADDR 失败: {}", e))?;
        sock.bind(&bind_addr.into())
            .map_err(|e| format!("绑定 {} 失败（端口可能被占用）: {}", bind_addr, e))?;
        let fallback: SocketAddr = format!("{}:{}", target_ip, target_port)
            .parse()
            .map_err(|e| format!("目标地址非法 {}:{}: {}", target_ip, target_port, e))?;
        info!("[qgc] UDP 链路已绑定 {}，命令回退目标 {}", bind_addr, fallback);
        Ok(Self {
            socket: Mutex::new(sock.into()),
            learned: Mutex::new(None),
            fallback,
            local: bind_addr,
        })
    }

    /// 本地地址（供模拟器发送遥测指向）
    pub fn local_addr(&self) -> SocketAddr {
        self.local
    }

    /// 当前发送目标（学习对端或配置回退）
    pub fn send_target(&self) -> SocketAddr {
        self.learned
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .unwrap_or(self.fallback)
    }

    /// 学习对端地址（接收线程每次收包时调用，命令发送指向最近活跃飞控）
    pub fn learn(&self, peer: SocketAddr) {
        *self.learned.lock().unwrap_or_else(|e| e.into_inner()) = Some(peer);
    }

    /// 阻塞接收（调用方需设置读超时以便轮询停止信号）
    pub fn recv_from(&self, buf: &mut [u8]) -> io::Result<(usize, SocketAddr)> {
        let sock = self.socket.lock().unwrap_or_else(|e| e.into_inner());
        sock.recv_from(buf)
    }
}
