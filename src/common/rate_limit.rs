//! # 登录速率限制（内存滑动窗口）
//!
//! bcrypt 密码验证约耗时 100ms CPU，无速率限制时攻击者可刷爆 CPU（DoS）。
//! 本模块提供进程内滑动窗口限流：按 `IP:email` 键计数，60 秒窗口内最多
//! `MAX_ATTEMPTS` 次尝试，超限后拒绝并返回需等待秒数。
//!
//! # 说明
//!
//! - 单进程内存实现（部署为单 exe 单进程，无需分布式限流）
//! - 条目在键数超过阈值时自动清理过期数据，防止内存无限增长
//! - 登录成功调用 `clear` 清除该键的失败记录

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

/// 滑动窗口时长（60 秒）
const WINDOW: Duration = Duration::from_secs(60);
/// 窗口内允许的最大尝试次数
const MAX_ATTEMPTS: usize = 5;
/// 键数量超过该值时触发过期条目清理
const CLEANUP_THRESHOLD: usize = 10_000;

/// 单个键的尝试记录
struct Bucket {
    /// 窗口内的尝试时间戳（按时间升序）
    attempts: Vec<Instant>,
}

static LIMITER: OnceLock<Mutex<HashMap<String, Bucket>>> = OnceLock::new();

fn limiter() -> &'static Mutex<HashMap<String, Bucket>> {
    LIMITER.get_or_init(|| Mutex::new(HashMap::new()))
}

/// 记录一次尝试并检查是否超限
///
/// # 参数
/// - `key`: 限流键（如 `"{ip}:{email}"`）
///
/// # 返回值
/// - `Ok(())`：允许继续，本次尝试已记录
/// - `Err(retry_after_secs)`：已被限流（本次尝试不计数），返回需等待的秒数
pub fn check_and_record(key: &str) -> Result<(), u64> {
    let now = Instant::now();
    let mut map = limiter().lock().unwrap_or_else(|e| e.into_inner());

    let bucket = map
        .entry(key.to_string())
        .or_insert_with(|| Bucket { attempts: Vec::new() });
    // 清理窗口外的旧尝试（滑动窗口）
    bucket.attempts.retain(|t| now.duration_since(*t) < WINDOW);

    if bucket.attempts.len() >= MAX_ATTEMPTS {
        let oldest = bucket
            .attempts
            .first()
            .map(|t| now.duration_since(*t))
            .unwrap_or(WINDOW);
        let wait_secs = WINDOW.as_secs().saturating_sub(oldest.as_secs()).max(1);
        return Err(wait_secs);
    }

    bucket.attempts.push(now);

    // 键数超阈值时清理过期条目，防止内存无限增长
    if map.len() >= CLEANUP_THRESHOLD {
        map.retain(|_, b| b.attempts.last().is_some_and(|t| now.duration_since(*t) < WINDOW));
    }

    Ok(())
}

/// 登录成功后清除该键的失败记录
pub fn clear(key: &str) {
    if let Some(map) = LIMITER.get() {
        if let Ok(mut map) = map.lock() {
            map.remove(key);
        }
    }
}
