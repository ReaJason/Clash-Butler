use config::Config;
use config::ConfigError;
use config::File;
use serde::Deserialize;

use crate::clash::DelayTestConfig;
use crate::speedtest::SpeedTestConfig;

#[derive(Deserialize, Debug)]
#[allow(unused)]
pub struct Settings {
    pub fast_mode: bool,
    pub subs: Vec<String>,
    pub rename_node: bool,
    pub rename_pattern: String,
    pub need_add_pool: bool,
    pub test_group_size: usize,
    /// rename_node 开启时，网站连通性并发测试的 worker 数量
    #[serde(default = "default_rename_test_workers")]
    pub rename_test_workers: usize,
    pub pools: Vec<String>,
    pub connect_test: DelayTestConfig,
    pub speed_test: SpeedTestConfig,
}

fn default_rename_test_workers() -> usize {
    8
}

#[cfg(test)]
mod tests {
    use super::*;

    // 旧配置没有 rename_test_workers 字段时应回退到默认值
    #[test]
    fn test_default_rename_test_workers() {
        let toml = r#"
            fast_mode = false
            subs = []
            rename_node = true
            rename_pattern = "${COUNTRYCODE}"
            need_add_pool = false
            test_group_size = 50
            pools = []
            [connect_test]
            url = "http://www.google.com/generate_204"
            expected = 204
            timeout = 3000
            [speed_test]
            enabled = false
            url = "https://speed.cloudflare.com/__down?bytes=104857600"
            timeout = 3000
        "#;
        let settings: Settings = toml::from_str(toml).unwrap();
        assert_eq!(settings.rename_test_workers, default_rename_test_workers());
    }
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let settings = Config::builder()
            .add_source(File::with_name("conf/config.toml"))
            .build()?;
        settings.try_deserialize::<Settings>()
    }
}
