//! # 公共 INI 配置封装
//!
//! 封装 `configparser` 库，提供 INI 配置文件的加载与读取。
//! 原 fj200c_information / ftj1c 各自一份同构实现，统一收拢到此公共模块。
//!
//! 注意：**全局实例（`GLOBAL`）不在此模块**。两个角色的配置文件不同
//! （`config-fj200c_information.ini` / `config-ftj1c.ini`），各角色模块通过
//! `config_singleton!` 宏生成自己的 `static GLOBAL: OnceLock<Config>` 与
//! `global()` / `set_global()`（仅只读单例；需热替换的 fj200c_main 保持自实现）。

use configparser::ini::Ini;
use std::path::Path;
use tracing::info;

/// INI 配置文件的封装结构（新类型模式）
///
/// 只暴露 `get` / `get_or` 等必要的读取方法，隐藏底层 `Ini` 细节。
#[derive(Debug, Clone)]
pub struct Config {
    /// 底层 INI 解析器实例
    inner: Ini,
}

impl Config {
    /// 从指定路径加载 INI 配置文件
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let path_ref = path.as_ref();
        info!("Config::load: 开始加载配置文件: {:?}", path_ref);
        let mut inner = Ini::new();
        inner.load(path_ref)?;
        let sections: Vec<String> = inner
            .sections()
            .into_iter()
            .map(|s| s.to_string())
            .collect();
        info!(
            "Config::load: 加载完成, 包含 {} 个节(section): {:?}",
            sections.len(),
            sections
        );
        Ok(Config { inner })
    }

    /// 获取指定 section 下的键值（不存在时返回默认值）
    pub fn get_or(&self, section: &str, key: &str, default: &str) -> String {
        self.inner.get(section, key).unwrap_or(default.into())
    }
}

/// 生成角色的全局只读配置单例（`global()` / `set_global()`）
///
/// 各角色配置文件不同，但单例形态一致（`OnceLock<Config>` 只读，仅能设置一次）。
/// 展开为模块级 `static GLOBAL` 与两个 pub 函数；重复展开时需使用不同标识符。
#[macro_export]
macro_rules! config_singleton {
    ($global_ident:ident, $getter:ident, $setter:ident) => {
        /// 全局配置实例（惰性初始化，只读）
        static $global_ident: std::sync::OnceLock<crate::common::config::Config> =
            std::sync::OnceLock::new();

        /// 获取全局配置实例的只读引用（`set_global` 成功后返回 `Some`）
        pub fn $getter() -> Option<&'static crate::common::config::Config> {
            $global_ident.get()
        }

        /// 设置全局配置实例（只能成功调用一次，重复设置返回 `Err(cfg)` 交还所有权）
        pub fn $setter(cfg: crate::common::config::Config) -> Result<(), crate::common::config::Config> {
            $global_ident.set(cfg)
        }
    };
}
