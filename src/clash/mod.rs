use std::collections::HashMap;
use std::fs::OpenOptions;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tracing::info;

pub async fn run() -> Child {
    let clash_path = "clash-meta/mihomo";
    if !Path::new(clash_path).exists() {
        panic!("无法找到 {} 启动文件，请将 mihomo 下载到当前的 clash-meta 目录下", clash_path);
    }

    let log_file = OpenOptions::new()
        .create(true)  // 如果文件不存在，则创建
        .write(true)   // 打开文件用于写入
        .append(true)  // 追加到文件而不是覆盖
        .open("logs/clash.log")
        .expect("Failed to open or create log file");

    let clash_process = Command::new(clash_path)
        .arg("-d")
        .arg("subs/test")
        .stdout(Stdio::from(log_file.try_clone().expect("Failed to clone log file handle")))
        .stderr(Stdio::from(log_file))
        .spawn().expect("fail to run mihomo");

    tokio::time::sleep(Duration::from_secs(1)).await;

    let response = reqwest::get("http://localhost:9090/version").await.unwrap();
    let version = response.json::<ClashVersion>().await.unwrap().version;

    info!("原神启动！ {}", version);

    clash_process
}

#[derive(Deserialize, Debug)]
struct ClashVersion {
    // meta: bool,
    version: String,
}

#[tokio::test]
async fn test_run_clash_meta() {
    run().await;
}

pub async fn get_proxies() -> Vec<Proxy> {
    let url = "http://127.0.0.1:9090/providers/proxies/%E8%87%AA%E5%8A%A8%E9%80%89%E6%8B%A9";

    let response = reqwest::get(url).await.unwrap();
    let provider = response.json::<Provider>().await.unwrap();

    provider.proxies
}

// #[warn(dead_code)]
// pub async fn restart() {
//     let client = Client::new();
//     let controller_api = "http://localhost:9090";
//     let url = format!("{}/restart", controller_api);
//
//     let body = json!({
//         "path": "subs/test",
//         "payload": ""
//     });
//
//     let response = client
//         .post(&url)
//         .json(&body)
//         .send()
//         .await.unwrap();
//
//     if response.status().is_success() {
//         info!("内核重启成功, {}", response.text().await.unwrap());
//     } else {
//         info!("内核重启失败: {}", response.status());
//     }
// }
//
// #[tokio::test]
// async fn test_restart_clash() {
//     restart().await
// }

#[tokio::test]
async fn test_get_proxies_delay() {
    let proxies = get_proxies().await;
    assert!(proxies.len() > 0)
}


#[derive(Deserialize)]
pub struct Provider {
    pub name: String,
    #[serde(rename = "testUrl")]
    pub test_url: String,
    pub proxies: Vec<Proxy>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Proxy {
    pub alive: bool,
    pub extra: HashMap<String, Extra>,
    pub history: Vec<History>,
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub proxy_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Extra {
    pub alive: bool,
    pub history: Vec<History>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct History {
    pub time: String,
    pub delay: u64,
}