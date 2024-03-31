use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::time::Duration;

use axum::{Router, routing::get};
use axum::extract::Query;
use regex::Regex;
use reqwest::Client;
use serde::Deserialize;
use tokio::net::TcpListener;
use tokio::signal;
use tower_http::services::ServeDir;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;
use walkdir::WalkDir;

mod sub;
mod clash;

#[tokio::main]
async fn main() {
    tracing::subscriber::set_global_default(FmtSubscriber::builder().with_max_level(Level::INFO).finish()).expect("setting default subscriber failed");

    create_logs_folder();
    sub::start_sub_converter().await;

    let app = Router::new()
        .route("/", get(root))
        .nest_service("/subs", ServeDir::new("subs"))
        .route("/add", get(add_sub))
        .route("/test", get(test_config))
        .route("/test/all", get(test_all_sub))
        ;

    let listener = TcpListener::bind("0.0.0.0:3000")
        .await
        .unwrap();

    info!("listening on {}", listener.local_addr().unwrap());

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await.unwrap();
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
        let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
        let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}


// 创建 logs 日志文件目录
fn create_logs_folder() {
    if Path::new("logs").exists() {
        return;
    }
    fs::create_dir("logs").unwrap()
}

#[test]
fn test_create_logs_folder() {
    create_logs_folder()
}

async fn root() -> &'static str {
    "Hello, World!"
}

#[derive(Deserialize, Debug)]
struct Sub {
    url: String,
}

async fn add_sub(Query(params): Query<Sub>) -> String {
    let sub_url = params.url;

    let sub_path = download_new_sub(&sub_url).await;
    let test_sub_url = format!("http://127.0.0.1:25500/sub?target=clash\
                                        &url=http://localhost:3000/subs/release/config.yaml%7Chttp://localhost:3000/{}\
                                        &config=config/clash-test.toml", sub_path);
    download_test_sub(&test_sub_url).await;

    exclude_test_duplicate_nodes().await;

    test_config().await;

    "http://localhost:3000/subs/release/config.yaml".to_string()
}

async fn test_config() -> String {
    let mut clash_process = clash::run().await;
    info!("开始测试 subs/test/config.yaml 中节点的延迟速度");

    let exclude_nodes = check_and_get_useless_nodes().await;

    let release_config_url = "https://gist.githubusercontent.com/ReaJason/633414d3b39af7dbbfcbdc08c8093d47/raw/a7d322b7c195db716435e93ca0b3a33d9c65a90b/gistfile1.txt";
    let release_sub_url = format!("http://127.0.0.1:25500/sub?target=clash\
                                            &url=http://localhost:3000/subs/test/config.yaml\
                                            &config={}\
                                            &exclude={}",
                                  release_config_url,
                                  urlencoding::encode(&*exclude_nodes.join("|")));

    let release_path = "subs/release/config.yaml";
    fs::copy(release_path, "subs/release/config.yaml.bak").unwrap();

    let release_file = File::create(release_path).unwrap();
    save_file_from_url(&release_sub_url, release_file).await;

    clash_process.kill().unwrap();
    clash_process.wait().unwrap();

    "http://localhost:3000/subs/release/config.yaml".to_string()
}

async fn test_all_sub() -> String {
    let mut sub_urls = Vec::new();

    sub_urls.push("http://localhost:3000/subs/release/config.yaml".to_string());

    // 遍历 `subs` 目录下的所有文件
    for entry in WalkDir::new("subs").into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_file() {
            if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                sub_urls.push(format!("http://localhost:3000/subs/{}", file_name));
            }
        }
    }
    info!("当前一共订阅数量为：{}", sub_urls.len());
    info!("{:?}", sub_urls);

    let test_sub_url = format!("http://127.0.0.1:25500/sub?target=clash\
                                        &url={}\
                                        &config=config/clash-test.toml",
                               urlencoding::encode(&sub_urls.join("|")));

    download_test_sub(&test_sub_url).await;

    exclude_test_duplicate_nodes().await;

    test_config().await;

    "http://localhost:3000/subs/release/config.yaml".to_string()
}

async fn exclude_test_duplicate_nodes() {
    let mut clash_process = clash::run().await;
    let proxies = clash::get_proxies().await;
    info!("当前节点个数：{}", proxies.len());

    let mut exclude_nodes = vec![];
    for proxy in proxies {
        let name = &proxy.name;
        let re = Regex::new(r"\s\d+$").unwrap();
        if re.is_match(name) {
            info!("去重节点：{}", name);
            exclude_nodes.push(name.clone())
        }
    }

    let test_sub_url = format!("http://127.0.0.1:25500/sub?target=clash\
                                    &url=http://localhost:3000/subs/test/config.yaml\
                                    &config=config/clash-test.toml&exclude={}",
                               urlencoding::encode(&exclude_nodes.join("|")));
    download_test_sub(&test_sub_url).await;
    info!("节点去重成功，去除个数 {}", exclude_nodes.len());
    clash_process.kill().unwrap();
    clash_process.wait().unwrap();
}

async fn check_and_get_useless_nodes() -> Vec<String> {
    let mut exclude_nodes = vec![];
    let mut round = 0;
    let total_round = 6;
    loop {
        let proxies = clash::get_proxies().await;
        // 当测试数量大于 5 时开始计算
        let cur_round = proxies.last().unwrap().history.len();
        if cur_round != round {
            round = cur_round;
            info!("当前已测完 {} 轮，一共测试 {} 轮", round, total_round);
        }

        if cur_round >= total_round {
            for proxy in proxies {
                let name = &proxy.name;
                let history = &proxy.history;
                let delays: std::collections::HashSet<_> = history.iter().map(|h| h.delay).collect();
                if delays.len() == 1 && *delays.iter().next().unwrap() == 0 {
                    info!("去掉全程无速度节点：{}", name);
                    exclude_nodes.push(name.clone());
                }

                if history.len() >= 2
                    && history[history.len() - 1].delay == 0
                    && history[history.len() - 2].delay == 0 {
                    info!("去掉多次连接无速度节点：{}", name);
                    exclude_nodes.push(name.clone());
                };
            }
            break;
        }
        tokio::time::sleep(Duration::from_secs(5)).await;
    }

    exclude_nodes
}

async fn download_test_sub(sub_url: &str) {
    let client = Client::new();
    let response = client.get(sub_url).send().await.unwrap();
    let content = response.text().await.unwrap();
    let mut file = File::create("subs/test/config.yaml").unwrap();
    file.write_all(content.as_bytes()).unwrap();
}

async fn download_new_sub(sub_url: &str) -> String {
    // 获取 UUID 作为文件名
    let re = Regex::new(r"/files/(.*?)/raw").unwrap();
    let uuid = re.captures(&sub_url)
        .and_then(|caps| caps.get(1))
        .map_or_else(|| {
            format!("{:x}", md5::compute(&sub_url))
        }, |m| m.as_str().to_string());

    let file_path = format!("subs/{}", uuid);
    info!("sub download success in {}", file_path);
    let file = File::create(&file_path).unwrap();

    save_file_from_url(sub_url, file).await;

    file_path
}

async fn save_file_from_url(url: &str, mut file: File) {
    let client = Client::new();
    let response = client.get(url).send().await.unwrap();
    let content = response.text().await.unwrap();
    file.write_all(content.as_bytes()).unwrap();
}

#[tokio::test]
async fn test_download_new_sub() {
    let sub_url = "https://paste.gg/p/anonymous/c89744fd11cc4f439881cd15d46c9548/files/87bb97abfb954e80a83619d677a2231c/raw";
    let sub_path = download_new_sub(sub_url).await;
    assert!(Path::new(&sub_path).exists());
    let sub_url = "http://localhost:3000/subs/1.yaml";
    let sub_path = download_new_sub(sub_url).await;
    assert!(Path::new(&sub_path).exists());
}