//! # 全局 KV 存储
//!
//! 基于 `OnceLock` + `RwLock` 的线程安全全局键值存储。
//! 用于在应用生命周期内共享少量配置/状态（如试验信息）。

use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};
use tracing::info;

static GLOBAL: OnceLock<GlobalVar> = OnceLock::new();

#[derive(Debug)]
pub struct GlobalVar {
    inner: RwLock<HashMap<String, String>>,
}

impl GlobalVar {
    pub fn init() {
        if GLOBAL.get().is_some() {
            info!("GlobalVar::init: 已初始化，跳过");
            return;
        }
        let this = Self {
            inner: RwLock::new(HashMap::new()),
        };
        GLOBAL.set(this).expect("GlobalVar::init: 初始化失败");
        info!("GlobalVar::init: 全局变量存储已初始化");
    }

    pub fn global() -> Option<&'static GlobalVar> {
        GLOBAL.get()
    }

    pub fn set(&self, key: &str, value: &str) {
        let mut map = self
            .inner
            .write()
            .expect("GlobalVar::set: RwLock 写入锁已损坏");
        map.insert(key.to_string(), value.to_string());
        info!("GlobalVar::set: 设置变量 [{}] = [{}]", key, value);
    }

    pub fn get(&self, key: &str) -> Option<String> {
        let map = self
            .inner
            .read()
            .expect("GlobalVar::get: RwLock 读取锁已损坏");
        map.get(key).cloned()
    }

    pub fn get_or(&self, key: &str, default: &str) -> String {
        let map = self
            .inner
            .read()
            .expect("GlobalVar::get_or: RwLock 读取锁已损坏");
        map.get(key)
            .cloned()
            .unwrap_or_else(|| default.to_string())
    }

    pub fn delete(&self, key: &str) -> bool {
        let mut map = self
            .inner
            .write()
            .expect("GlobalVar::delete: RwLock 写入锁已损坏");
        let removed = map.remove(key);
        if removed.is_some() {
            info!("GlobalVar::delete: 已删除变量 [{}]", key);
            true
        } else {
            false
        }
    }

    pub fn contains(&self, key: &str) -> bool {
        let map = self
            .inner
            .read()
            .expect("GlobalVar::contains: RwLock 读取锁已损坏");
        map.contains_key(key)
    }

    pub fn keys(&self) -> Vec<String> {
        let map = self
            .inner
            .read()
            .expect("GlobalVar::keys: RwLock 读取锁已损坏");
        map.keys().cloned().collect()
    }

    pub fn snapshot(&self) -> HashMap<String, String> {
        let map = self
            .inner
            .read()
            .expect("GlobalVar::snapshot: RwLock 读取锁已损坏");
        map.clone()
    }

    pub fn clear(&self) {
        let mut map = self
            .inner
            .write()
            .expect("GlobalVar::clear: RwLock 写入锁已损坏");
        map.clear();
        info!("GlobalVar::clear: 已清空所有变量");
    }

    pub fn len(&self) -> usize {
        let map = self
            .inner
            .read()
            .expect("GlobalVar::len: RwLock 读取锁已损坏");
        map.len()
    }

    pub fn is_empty(&self) -> bool {
        let map = self
            .inner
            .read()
            .expect("GlobalVar::is_empty: RwLock 读取锁已损坏");
        map.is_empty()
    }
}
