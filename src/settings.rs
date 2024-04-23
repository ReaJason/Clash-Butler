use std::collections::HashMap;
use config::{Config, ConfigError, File};
use serde::Deserialize;
use crate::clash::DelayTestConfig;

#[derive(Deserialize, Debug)]
#[allow(unused)]
pub struct Settings {
    pub fast_mode: bool,
    pub test: Option<bool>,
    pub subs: Vec<String>,
    pub sub_config_url: String,
    pub rename_node: bool,
    pub rename_pattern: String,
    pub need_add_pool: bool,
    pub pools: Vec<String>,
    pub connect_test: DelayTestConfig,
    pub websites: HashMap<String, DelayTestConfig>,
    pub other: Other,
}

#[derive(Deserialize, Debug)]
#[allow(unused)]
pub struct Other {
    pub sub_converter_port: u64,
    pub clash_external_port: u64,
}


impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let settings = Config::builder()
            .add_source(File::with_name("config.toml"))
            .build()?;
        settings.try_deserialize::<Settings>()
    }
}
