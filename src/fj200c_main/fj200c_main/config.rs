use std::sync::OnceLock;

pub use crate::common::config::Config;

static GLOBAL: OnceLock<Config> = OnceLock::new();

pub fn global() -> Option<&'static Config> {
    GLOBAL.get()
}

pub fn set_global(cfg: Config) -> Result<(), Config> {
    GLOBAL.set(cfg)
}
