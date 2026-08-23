//! # 可逆加密模块
//!
//! 提供 AES-256-GCM 可逆加密与 HMAC-SHA256 指纹计算，用于防止直接读取 SQLite
//! 文件即可看到敏感明文：
//!
//! | 用途 | 说明 |
//! |------|------|
//! | `encrypt` / `decrypt` | 对数据库中的**用户名**与**种子账号初始密码**做可逆加密 |
//! | `username_hash` | 对用户名计算确定性指纹，用于唯一约束、查重与表 join |
//!
//! # 设计说明
//!
//! - **密钥来源**：沿用 `JWT_SECRET`。通过 SHA-256 派生 32 字节 AES 密钥；
//!   HMAC 指纹使用"域分隔"后的独立 32 字节密钥（避免同一密钥既加密又做指纹）。
//!   因此 **`JWT_SECRET` 变更会使旧密文无法解密**（属预期行为，见 `decrypt` 的错误提示）。
//! - **加密格式**：`hex(nonce(12B) || ciphertext || tag(16B))`，每次加密生成随机 nonce，
//!   同一明文每次密文不同（GCM 认证加密，可检测篡改）。
//! - **为什么不直接用 bcrypt**：bcrypt 带随机盐、单向且每次结果不同，无法满足
//!   数据库唯一约束与按用户名查重/join 的需求，故用户名采用"可逆加密 + 指纹"组合。

use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM};
use ring::digest;
use ring::hmac;
use ring::rand::{SecureRandom, SystemRandom};
use std::sync::OnceLock;

/// AES-256 密钥（由 JWT_SECRET 经 SHA-256 派生，全局缓存）
static AES_KEY: OnceLock<[u8; 32]> = OnceLock::new();
/// HMAC 指纹密钥（域分隔派生，与 AES 密钥不同，全局缓存）
static HMAC_KEY: OnceLock<[u8; 32]> = OnceLock::new();

/// 派生并缓存 AES-256 密钥
///
/// 密钥 = SHA-256(JWT_SECRET)。调用方需保证 `jwt::init()` 已执行（`main.rs` 启动时先于建库）。
fn aes_key() -> &'static [u8; 32] {
    AES_KEY.get_or_init(|| {
        let d = digest::digest(&digest::SHA256, crate::common::jwt::secret().as_bytes());
        let mut k = [0u8; 32];
        k.copy_from_slice(d.as_ref());
        k
    })
}

/// 派生并缓存 HMAC 指纹密钥
///
/// 密钥 = SHA-256("username-hmac:" + JWT_SECRET)，与 AES 密钥域分隔，避免密钥复用。
fn hmac_key() -> &'static [u8; 32] {
    HMAC_KEY.get_or_init(|| {
        let mut ctx = digest::Context::new(&digest::SHA256);
        ctx.update(b"username-hmac:");
        ctx.update(crate::common::jwt::secret().as_bytes());
        let mut k = [0u8; 32];
        k.copy_from_slice(ctx.finish().as_ref());
        k
    })
}

/// 十六进制编码工具：字节数组 → 小写十六进制字符串
fn to_hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        let _ = write!(s, "{:02x}", b);
    }
    s
}

/// 十六进制解码工具：十六进制字符串 → 字节数组（校验长度）
fn from_hex(hex: &str) -> Result<Vec<u8>, String> {
    let mut out = Vec::with_capacity(hex.len() / 2);
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
    for chunk in b.chunks(2) {
        out.push(val(chunk[0])? * 16 + val(chunk[1])?);
    }
    Ok(out)
}

/// 可逆加密（AES-256-GCM）
///
/// # 返回
/// `hex(nonce(12B) || ciphertext || tag(16B))`。同一明文每次结果不同（随机 nonce）。
///
/// # 参数
/// - `plaintext`: 待加密的明文（用户名 / 初始密码）
///
/// # 返回值
/// - `Ok(String)` - 加密后的十六进制密文
/// - `Err(...)` - 加密失败（密钥派生或随机数生成错误）
pub fn encrypt(plaintext: &str) -> Result<String, String> {
    let key = UnboundKey::new(&AES_256_GCM, aes_key()).map_err(|e| e.to_string())?;
    let key = LessSafeKey::new(key);

    // 生成 12 字节随机 nonce
    let rng = SystemRandom::new();
    let mut nonce_bytes = [0u8; 12];
    rng.fill(&mut nonce_bytes).map_err(|_| "随机数生成失败")?;
    let nonce = Nonce::assume_unique_for_key(nonce_bytes);

    // 在明文后追加 GCM tag（in_out 之内完成认证加密）
    let mut in_out = plaintext.as_bytes().to_vec();
    key.seal_in_place_append_tag(nonce, Aad::empty(), &mut in_out)
        .map_err(|_| "AES-256-GCM 加密失败")?;

    // 组装 nonce || cipertext || tag，再十六进制编码
    let mut out = Vec::with_capacity(nonce_bytes.len() + in_out.len());
    out.extend_from_slice(&nonce_bytes);
    out.extend_from_slice(&in_out);
    Ok(to_hex(&out))
}

/// 可逆解密（AES-256-GCM）
///
/// # 参数
/// - `blob`: `encrypt` 产出的十六进制密文
///
/// # 返回值
/// - `Ok(String)` - 解密后的明文
/// - `Err(...)` - 密文非法 / 篡改 / 密钥不符（如 `JWT_SECRET` 变更）
pub fn decrypt(blob: &str) -> Result<String, String> {
    let raw = from_hex(blob)?;
    // 至少需要 nonce(12) + tag(16)
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
        .map_err(|_| "解密失败：密文被篡改或 JWT_SECRET 已变更")?;
    String::from_utf8(plaintext.to_vec()).map_err(|e| e.to_string())
}

/// 解密，失败时回退返回原值
///
/// 用于兼容数据库中的**历史明文**（未迁移行）或无法解密的旧数据，避免读到
/// 空/报错。迁移完成后所有行均为密文，正常情况下不会走回退分支。
pub fn decrypt_or_plaintext(blob: &str) -> String {
    decrypt(blob).unwrap_or_else(|_| blob.to_string())
}

/// 用户名字符串指纹（HMAC-SHA256 十六进制，确定性）
///
/// 同一用户名结果恒定，供数据库唯一约束、查重与 `users` / `seed_passwords` 两表 join。
/// 不用 `encrypt` 的原因：AES-GCM 每次结果不同，无法做等值比较与索引查询。
///
/// # 参数
/// - `username`: 用户名字符串
///
/// # 返回值
/// 64 位小写十六进制指纹
pub fn username_hash(username: &str) -> String {
    let key = hmac::Key::new(hmac::HMAC_SHA256, hmac_key());
    let tag = hmac::sign(&key, username.as_bytes());
    to_hex(tag.as_ref())
}