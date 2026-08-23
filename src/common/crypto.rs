//! # 可逆加密模块
//!
//! 提供 AES-256-GCM 可逆加密与 HMAC-SHA256 指纹计算，用于防止直接读取 SQLite
//! 文件即可看到敏感明文（用户名 / 邮箱 / 角色 / 种子初始密码）：
//!
//! | 用途 | 说明 |
//! |------|------|
//! | `encrypt` / `decrypt` | 对库中的用户名、邮箱、角色、初始密码做可逆加密（含防篡改认证） |
//! | `username_hash` / `field_hash` | 对用户名/邮箱计算确定性指纹，用于唯一约束、查重与登录定位 |
//!
//! # 设计说明
//!
//! - **密钥分离**：本模块使用的 **数据加密密钥（`DATA_ENCRYPTION_KEY`）** 与 JWT 签名密钥
//!   完全独立。前者由**本机 MAC 地址自动派生**（`data_key()`：SHA-256(MAC)），后者硬编码进
//!   `jwt.rs`。两者互不相干：JWT 密钥泄露不影响磁盘密文，MAC 变更才会使旧密文失效
//!   （属预期行为，等同换机无法解密旧库）。
//! - **派生方式**：`data_key()` 先取本机首个物理网卡 MAC 地址（Windows `getmac` /
//!   Linux `/sys/class/net`），SHA-256 得到 32 字节数据密钥；AES-256 密钥直接取该值，
//!   HMAC 指纹密钥用"域分隔"前缀 `field-hmac:` 再 SHA-256 一次（避免同一密钥既加密又做指纹）。
//! - **加密格式**：`hex(nonce(12B) || ciphertext || tag(16B))`，每次随机 nonce，同一明文密文
//!   不同；GCM 认证标签可检测篡改。
//! - **为什么不用 bcrypt**：bcrypt 单向且每次结果不同，无法做登录按邮箱等值查找与唯一约束，
//!   故用户名/邮箱用"可逆加密 + 指纹"组合。

use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM};
use ring::digest;
use ring::hmac;
use ring::rand::{SecureRandom, SystemRandom};
use std::sync::OnceLock;

/// AES-256 密钥（由本机 MAC 派生 DATA_ENCRYPTION_KEY 经 SHA-256，全局缓存）
static AES_KEY: OnceLock<[u8; 32]> = OnceLock::new();
/// HMAC 指纹密钥（域分隔派生，与 AES 密钥不同，全局缓存）
static HMAC_KEY: OnceLock<[u8; 32]> = OnceLock::new();

/// 取本机首个物理网卡 MAC 地址字符串（跨平台，回退到主机名）
///
/// - Windows：`getmac /fo csv /nh` 解析首列 MAC（形如 `00-1B-44-11-3A-B5`）
/// - Linux：`/sys/class/net/<if>/address`（跳过 `lo`）
/// - 其他/失败：回退到 `COMPUTERNAME`/`HOSTNAME`，保证至少有一个稳定密钥源
fn machine_mac() -> String {
    #[cfg(windows)]
    {
        if let Ok(out) = std::process::Command::new("getmac")
            .args(["/fo", "csv", "/nh"])
            .output()
        {
            if out.status.success() {
                let s = String::from_utf8_lossy(&out.stdout);
                for line in s.lines() {
                    let first = line.split(',').next().unwrap_or("").trim().trim_matches('"');
                    if !first.is_empty() && first != "00-00-00-00-00-00" {
                        return first.to_string();
                    }
                }
            }
        }
    }
    #[cfg(target_os = "linux")]
    {
        if let Ok(entries) = std::fs::read_dir("/sys/class/net") {
            for entry in entries.flatten() {
                let name = entry.file_name();
                if name == "lo" {
                    continue;
                }
                let addr_path = entry.path().join("address");
                if let Ok(addr) = std::fs::read_to_string(addr_path) {
                    let addr = addr.trim();
                    if !addr.is_empty() && addr != "00:00:00:00:00:00" {
                        return addr.to_string();
                    }
                }
            }
        }
    }
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "rustweb-default-machine".to_string())
}

/// 数据加密密钥 `DATA_ENCRYPTION_KEY`（由本机 MAC 地址 SHA-256 派生，全局缓存）
///
/// 与 JWT 签名密钥完全独立，仅用于本模块的字段加解密与指纹。MAC 变更会使旧密文
/// 无法解密（预期行为）。需先调用 `jwt::init()`（`main.rs` 启动时先于建库执行）。
pub(crate) fn data_key() -> &'static [u8; 32] {
    static DATA_KEY: OnceLock<[u8; 32]> = OnceLock::new();
    DATA_KEY.get_or_init(|| {
        let mac = machine_mac();
        tracing::debug!("数据加密密钥由本机 MAC 地址派生：{}", mac);
        let d = digest::digest(&digest::SHA256, mac.as_bytes());
        let mut k = [0u8; 32];
        k.copy_from_slice(d.as_ref());
        k
    })
}

/// 派生并缓存 AES-256 密钥
///
/// 密钥 = SHA-256(DATA_ENCRYPTION_KEY) = SHA-256(SHA-256(MAC))。
fn aes_key() -> &'static [u8; 32] {
    AES_KEY.get_or_init(|| {
        let d = digest::digest(&digest::SHA256, data_key());
        let mut k = [0u8; 32];
        k.copy_from_slice(d.as_ref());
        k
    })
}

/// 派生并缓存 HMAC 指纹密钥
fn hmac_key() -> &'static [u8; 32] {
    HMAC_KEY.get_or_init(|| {
        let mut ctx = digest::Context::new(&digest::SHA256);
        ctx.update(b"field-hmac:");
        ctx.update(data_key());
        let mut k = [0u8; 32];
        k.copy_from_slice(ctx.finish().as_ref());
        k
    })
}

/// 字节数组 → 小写十六进制字符串
fn to_hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        let _ = write!(s, "{:02x}", b);
    }
    s
}

/// 十六进制字符串 → 字节数组
fn from_hex(hex: &str) -> Result<Vec<u8>, String> {
    let b = hex.as_bytes();
    if b.len() % 2 != 0 {
        return Err("十六进制字符串长度必须为偶数".into());
    }
    let val = |c: u8| -> Result<u8, String> {
        match c {
            b'0'..=b'9' => Ok(c - b'0'),
            b'a'..=b'f' => Ok(c - b'a' + 10),
            b'A'..=b'F' => Ok(c - b'A' + 10),
            _ => Err("非法十六进制字符".into()),
        }
    };
    let mut out = Vec::with_capacity(b.len() / 2);
    for chunk in b.chunks(2) {
        out.push(val(chunk[0])? * 16 + val(chunk[1])?);
    }
    Ok(out)
}

/// 可逆加密（AES-256-GCM）
///
/// # 返回
/// `hex(nonce(12B) || ciphertext || tag(16B))`，同一明文每次结果不同（随机 nonce）。
///
/// # 参数
/// - `plaintext`: 待加密的明文（用户名 / 邮箱 / 角色 / 初始密码）
pub fn encrypt(plaintext: &str) -> Result<String, String> {
    let key = UnboundKey::new(&AES_256_GCM, aes_key()).map_err(|e| e.to_string())?;
    let key = LessSafeKey::new(key);

    let rng = SystemRandom::new();
    let mut nonce_bytes = [0u8; 12];
    rng.fill(&mut nonce_bytes).map_err(|_| "随机数生成失败")?;
    let nonce = Nonce::assume_unique_for_key(nonce_bytes);

    // 明文后追加 GCM 认证标签（in_out 内完成认证加密）
    let mut in_out = plaintext.as_bytes().to_vec();
    key.seal_in_place_append_tag(nonce, Aad::empty(), &mut in_out)
        .map_err(|_| "AES-256-GCM 加密失败")?;

    let mut out = Vec::with_capacity(nonce_bytes.len() + in_out.len());
    out.extend_from_slice(&nonce_bytes);
    out.extend_from_slice(&in_out);
    Ok(to_hex(&out))
}

/// 可逆解密（AES-256-GCM）
///
/// # 返回
/// - `Ok(String)` - 解密明文
/// - `Err(...)` - 密文非法 / 被篡改 / 密钥不符（如本机 MAC 地址已变更）
pub fn decrypt(blob: &str) -> Result<String, String> {
    let raw = from_hex(blob)?;
    if raw.len() < 12 + 16 {
        return Err("密文过短或格式非法".into());
    }
    let (nonce_bytes, ct_bytes) = raw.split_at(12);
    let key = UnboundKey::new(&AES_256_GCM, aes_key()).map_err(|e| e.to_string())?;
    let key = LessSafeKey::new(key);
    let nonce = Nonce::try_assume_unique_for_key(nonce_bytes).map_err(|_| "nonce 非法")?;

    let mut in_out = ct_bytes.to_vec();
    let plaintext = key
        .open_in_place(nonce, Aad::empty(), &mut in_out)
        .map_err(|_| "解密失败：密文被篡改或本机 MAC 地址已变更（DATA_ENCRYPTION_KEY 失配）")?;
    String::from_utf8(plaintext.to_vec()).map_err(|e| e.to_string())
}

/// 解密，失败时回退返回原值
///
/// 兼容数据库中未迁移的旧明文行或无法解密的旧数据，避免读取报错。
pub fn decrypt_or_plaintext(blob: &str) -> String {
    decrypt(blob).unwrap_or_else(|_| blob.to_string())
}

/// 用户名指纹（HMAC-SHA256 十六进制，确定性）
pub fn username_hash(username: &str) -> String {
    field_hash(username)
}

/// 任意字段指纹（HMAC-SHA256 十六进制，确定性）
///
/// 与 `username_hash` 同算法，供邮箱等需要查重 / 登录定位的字段生成指纹
/// （如 `users.email_hash`，唯一约束承载在指纹唯一索引上）。
pub fn field_hash(value: &str) -> String {
    let key = hmac::Key::new(hmac::HMAC_SHA256, hmac_key());
    let tag = hmac::sign(&key, value.as_bytes());
    to_hex(tag.as_ref())
}