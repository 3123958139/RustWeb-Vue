//! # 公共 INI 配置封装
//!
//! 封装 `configparser` 库，提供 INI 配置文件的加载与读取。
//! 原 fj200c_information / ftj1c 各自一份同构实现，统一收拢到此公共模块。
//!
//! 注意：**全局实例（`GLOBAL`）不在此模块**。两个角色的配置文件不同
//! （`config-fj200c_information.ini` / `config-ftj1c.ini`），各角色模块各自维护
//! 自己的 `static GLOBAL: OnceLock<Config>` 与 `global()` / `set_global()`。

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
