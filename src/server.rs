#![allow(unused)]

use std::fs;
use std::fs::File;
use std::io::Write;
use std::time::Duration;

use axum::extract::Query;
use axum::Router;
use axum::routing::get;
use regex::Regex;
use reqwest::Client;
use serde::Deserialize;
use tokio::net::TcpListener;
use tokio::signal;
use tower_http::services::ServeDir;
use tracing::info;
use walkdir::WalkDir;

use crate::{clash, routes, Settings};

pub async fn start_server(_config: Settings) {
    let app = Router::new()
        .route("/", get(root))
        .nest_service("/subs", ServeDir::new("subs"))
        // .route("/add", get(add_sub))
        // .route("/test", get(test_config))
        // .route("/test/all", get(test_all_sub))
        .merge(routes::sub::sub_router())
        .merge(routes::config::config_router())
        ;

    let listener = TcpListener::bind("0.0.0.0:3003")
        .await
        .unwrap();

    info!("listening on {}", listener.local_addr().unwrap());

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await.unwrap();
}


async fn root() -> &'static str {
    "👋 Clash-Butler!"
}
//
// #[derive(Deserialize, Debug)]
// struct Sub {
//     url: String,
// }
//
// async fn add_sub(Query(params): Query<Sub>) -> String {
//     let sub_url = params.url;
//
//     let sub_path = download_new_sub(&sub_url).await;
//     let test_sub_url = format!("http://127.0.0.1:25500/sub?target=clash\
//                                         &url=http://localhost:3000/subs/release/config.yaml%7Chttp://localhost:3000/{}\
//                                         &config=config/clash-test.toml", sub_path);
//     download_test_sub(&test_sub_url).await;
//
//     exclude_test_duplicate_nodes().await;
//
//     test_config().await;
//
//     "http://localhost:3000/subs/release/config.yaml".to_string()
// }
//
// async fn test_config() -> String {
//     let mut clash_process = clash::start().await;
//     info!("开始测试 subs/test/config.yaml 中节点的延迟速度");
//
//     let exclude_nodes = check_and_get_useless_nodes().await;
//
//     let release_config_url = "https://gist.githubusercontent.com/ReaJason/633414d3b39af7dbbfcbdc08c8093d47/raw/a7d322b7c195db716435e93ca0b3a33d9c65a90b/gistfile1.txt";
//     let release_sub_url = format!("http://127.0.0.1:25500/sub?target=clash\
//                                             &url=http://localhost:3003/subs/test/config.yaml\
//                                             &config={}\
//                                             &exclude={}",
//                                   release_config_url,
//                                   urlencoding::encode(&exclude_nodes.join("|")));
//
//     let release_path = "subs/release/config.yaml";
//     fs::copy(release_path, "subs/release/config.yaml.bak").unwrap();
//
//     let release_file = File::create(release_path).unwrap();
//     save_file_from_url(&release_sub_url, release_file).await;
//
//     clash_process.kill().unwrap();
//     clash_process.wait().unwrap();
//
//     "http://localhost:3000/subs/release/config.yaml".to_string()
// }
//
// async fn test_all_sub() -> String {
//     let mut sub_urls = Vec::new();
//
//     sub_urls.push("http://localhost:3000/subs/release/config.yaml".to_string());
//
//     // 遍历 `subs` 目录下的所有文件
//     for entry in WalkDir::new("subs").into_iter().filter_map(|e| e.ok()) {
//         let path = entry.path();
//         if path.is_file() {
//             if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
//                 sub_urls.push(format!("http://localhost:3000/subs/{}", file_name));
//             }
//         }
//     }
//     info!("当前一共订阅数量为：{}", sub_urls.len());
//     info!("{:?}", sub_urls);
//
//     let test_sub_url = format!("http://127.0.0.1:25500/sub?target=clash\
//                                         &url={}\
//                                         &config=config/clash-test.toml",
//                                urlencoding::encode(&sub_urls.join("|")));
//
//     download_test_sub(&test_sub_url).await;
//
//     exclude_test_duplicate_nodes().await;
//
//     test_config().await;
//
//     "http://localhost:3000/subs/release/config.yaml".to_string()
// }
//
// async fn exclude_test_duplicate_nodes() {
//     let mut clash_process = clash::start().await;
//     let proxies = clash::get_provider_proxies().await;
//     info!("当前节点个数：{}", proxies.len());
//
//     let mut exclude_nodes = vec![];
//     for proxy in proxies {
//         let name = &proxy.name;
//         let re = Regex::new(r"\s\d+$").unwrap();
//         if re.is_match(name) {
//             info!("去重节点：{}", name);
//             exclude_nodes.push(name.clone())
//         }
//     }
//
//     let test_sub_url = format!("http://127.0.0.1:25500/sub?target=clash\
//                                     &url=http://localhost:3000/subs/test/config.yaml\
//                                     &config=config/clash-test.toml&exclude={}",
//                                urlencoding::encode(&exclude_nodes.join("|")));
//     download_test_sub(&test_sub_url).await;
//     info!("节点去重成功，去除个数 {}", exclude_nodes.len());
//     clash_process.kill().unwrap();
//     clash_process.wait().unwrap();
// }
//
// async fn check_and_get_useless_nodes() -> Vec<String> {
//     let mut exclude_nodes = vec![];
//     let mut round = 0;
//     let total_round = 6;
//     loop {
//         let proxies = clash::get_provider_proxies().await;
//         // 当测试数量大于 5 时开始计算
//         let cur_round = proxies.last().unwrap().history.len();
//         if cur_round != round {
//             round = cur_round;
//             info!("当前已测完 {} 轮，一共测试 {} 轮", round, total_round);
//         }
//
//         if cur_round >= total_round {
//             for proxy in proxies {
//                 let name = &proxy.name;
//                 let history = &proxy.history;
//                 let delays: std::collections::HashSet<_> = history.iter().map(|h| h.delay).collect();
//                 if delays.len() == 1 && *delays.iter().next().unwrap() == 0 {
//                     info!("去掉全程无速度节点：{}", name);
//                     exclude_nodes.push(name.clone());
//                 }
//
//                 if history.len() >= 2
//                     && history[history.len() - 1].delay == 0
//                     && history[history.len() - 2].delay == 0 {
//                     info!("去掉多次连接无速度节点：{}", name);
//                     exclude_nodes.push(name.clone());
//                 };
//             }
//             break;
//         }
//         tokio::time::sleep(Duration::from_secs(5)).await;
//     }
//
//     exclude_nodes
// }
//
// async fn download_test_sub(sub_url: &str) {
//     let client = Client::new();
//     let response = client.get(sub_url).send().await.unwrap();
//     let content = response.text().await.unwrap();
//     let mut file = File::create("subs/test/config.yaml").unwrap();
//     file.write_all(content.as_bytes()).unwrap();
// }
//
// async fn download_new_sub(sub_url: &str) -> String {
//     // 获取 UUID 作为文件名
//     let re = Regex::new(r"/files/(.*?)/raw").unwrap();
//     let uuid = re.captures(sub_url)
//         .and_then(|caps| caps.get(1))
//         .map_or_else(|| {
//             format!("{:x}", md5::compute(sub_url))
//         }, |m| m.as_str().to_string());
//
//     let file_path = format!("subs/{}", uuid);
//     info!("sub download success in {}", file_path);
//     let file = File::create(&file_path).unwrap();
//
//     save_file_from_url(sub_url, file).await;
//
//     file_path
// }
//
// async fn save_file_from_url(url: &str, mut file: File) {
//     let client = Client::new();
//     let response = client.get(url).send().await.unwrap();
//     let content = response.text().await.unwrap();
//     file.write_all(content.as_bytes()).unwrap();
// }
//
// #[tokio::test]
// async fn test_download_new_sub() {
//     let sub_urls = vec![
//         "https://paste.gg/p/anonymous/762b2a74a56d4d70aca32af09cb27166/files/279fdd53601641e0ba901f22d1be2f8a/raw",
//         "https://paste.gg/p/anonymous/160e38c0bbcf43888f0dd05d71d2f940/files/5c5ef84089e64bd18a9648ffa3008ef0/raw",
//         "https://paste.gg/p/anonymous/8d8892f5d51d4fb5a8b9de021b8c1203/files/716fa54fc574435da0f00bf39318b4a5/raw",
//         "https://paste.gg/p/anonymous/bf53b02670484b7a890c0104871ead37/files/02b4accb51854b5fb17025b08c898f6b/raw",
//         "https://paste.gg/p/anonymous/c3aeb7d1bd1f436981fb2e510ed2ad28/files/694b1678012c42ee8384ba763f54184e/raw",
//         "https://paste.gg/p/anonymous/325c3f3d3e4a4a55beb2da123d0c0a79/files/abc6307f9b0948fd87ce67f3b90e8c75/raw",
//         "https://paste.gg/p/anonymous/085d5d9bdbea4b2894ad3edfb4e641b0/files/b2e4f145d117443eae95c7446d0fc5d8/raw",
//         "https://paste.gg/p/anonymous/b0e54ca4c30748fe8b903bbd10d0265b/files/58f1e656b643412fa63a4afecfefdbb6/raw",
//         "https://paste.gg/p/anonymous/c0d64e2711974406a0302682d30d1528/files/047e16e0a716430fbea90b240fe20a0d/raw",
//         "https://paste.gg/p/anonymous/b48355063ace416786bfa41d9df4ccdf/files/670833ee40fb4542878db61bf81062ad/raw",
//         "https://paste.gg/p/anonymous/c030dcdd51604896af121d38b1535e14/files/d5da82eaae674203b070f79a28212b6a/raw",
//         "https://paste.gg/p/anonymous/5575895137714c3dace4021ebe91f053/files/dfd5431940d44be7b3fed1f84b1a4e4c/raw",
//     ];
//     for sub_url in sub_urls {
//         let sub_path = download_new_sub(sub_url).await;
//         assert!(Path::new(&sub_path).exists());
//     }
// }
//
//
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