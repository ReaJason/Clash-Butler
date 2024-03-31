use std::fs::OpenOptions;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use tracing::info;

// 启动 sub converter 服务
pub async fn start_sub_converter() -> Child {
    let sub_converter_starter_path = "subconverter/subconverter";
    if !Path::new(sub_converter_starter_path).exists() {
        panic!("无法找到 {} 启动文件，请将 subconverter 下载到当前的 subconverter 目录下", sub_converter_starter_path);
    }

    let log_file = OpenOptions::new()
        .create(true)  // 如果文件不存在，则创建
        .write(true)   // 打开文件用于写入
        .append(true)  // 追加到文件而不是覆盖
        .open("logs/sub.log")
        .expect("Failed to open or create log file");

    let sub_converter_process = Command::new(sub_converter_starter_path)
        .stdout(Stdio::from(log_file.try_clone().expect("Failed to clone log file handle")))
        .stderr(Stdio::from(log_file))
        .spawn()
        .expect("Failed to start subconverter service");

    tokio::time::sleep(Duration::from_secs(1)).await;

    match reqwest::get("http://127.0.0.1:25500/version").await {
        Ok(response) => {
            if response.status().is_success() {
                let version = response.text().await;
                info!("{}", version.unwrap().trim().to_owned() + " 服务已启动");
            } else {
                panic!("subconverter 服务启动失败");
            }
        }
        Err(_) => panic!("无法连接到 subconverter 服务"),
    }

    return sub_converter_process;
}

#[tokio::test]
async fn test_start_sub_converter() {
    let mut sub_converter_process = start_sub_converter().await;
    sub_converter_process.kill().unwrap();
    sub_converter_process.wait().unwrap();

    match reqwest::get("http://127.0.0.1:25500/version").await {
        Ok(_) => println!("subconverter 服务未正确关闭"),
        Err(_) => println!("subconverter 服务已关闭"),
    }
}