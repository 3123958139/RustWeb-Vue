use std::sync::{OnceLock, RwLock, RwLockReadGuard, RwLockWriteGuard};

pub use crate::common::config::Config;

static GLOBAL: OnceLock<RwLock<Option<Config>>> = OnceLock::new();

fn lock() -> &'static RwLock<Option<Config>> {
    GLOBAL.get_or_init(|| RwLock::new(None))
}

pub fn global() -> Option<RwLockReadGuard<'static, Option<Config>>> {
    lock().read().ok()
}

pub fn set_global(cfg: Config) {
    if let Ok(mut guard) = lock().write() {
        *guard = Some(cfg);
    }
}

pub fn clear_global() {
    if let Ok(mut guard) = lock().write() {
        *guard = None;
    }
}
