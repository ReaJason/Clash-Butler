use std::{env, fs};
use std::collections::{HashMap, HashSet};
use std::default::Default;
use std::fs::File;
use std::io::Write;
use std::net::IpAddr;
use std::path::Path;
use std::time::Duration;

use chrono::Local;
use clap::Parser;
use reqwest::Client;
use tracing::{error, info, Level};
use tracing_subscriber::FmtSubscriber;

use crate::clash::{ClashMeta, DelayTestConfig};
use crate::settings::Settings;
use crate::sub::{SubConfig, SubConverter};

mod sub;
mod clash;
mod routes;
mod risk;
mod server;
mod ip;
mod cgi_trace;
mod settings;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    // Starts the Axum server
    #[arg(long)]
    server: bool,

    // Just test subs/test/config.yaml
    #[arg(long)]
    test: bool,
}

const TEST_PROXY_NAME: &str = "PROXY";

#[tokio::main]
async fn main() {
    tracing::subscriber::set_global_default(
        FmtSubscriber::builder()
            .with_max_level(Level::INFO)
            .finish()
    ).expect("setting default subscriber failed");
    let args = Cli::parse();
    let config = Settings::new();
    match config {
        Ok(mut config) => {
            // 创建订阅测试所用的目录结构
            create_folder();
            if args.server {
                // 服务端
                // server::start_server(config).await
            } else {
                // 本地生成
                if args.test {
                    config.test = Some(true);
                }
                run(config).await
            }
        }
        Err(e) => {
            panic!("配置文件读取失败: {}", e)
        }
    }
}

async fn run(config: Settings) {
    // 启动 sub converter 服务
    let mut subconverter = SubConverter::new(config.other.sub_converter_port);
    if let Err(e) = subconverter.start().await {
        error!("subconverter 启动失败，{}", e);
        return;
    }

    let test_sub_file_path;
    let mixed_port = 7999;
    let external_port = config.other.clash_external_port;

    if config.test.is_some() {
        let config_path = env::current_dir().unwrap().join("subs/test/config.yaml");
        if !config_path.exists() {
            error!("当前并没有找到可用的测试文件，请删掉 --test 后重试");
            return;
        }
        test_sub_file_path = config_path.to_string_lossy().to_string();
    } else {
        let mut urls = config.subs;
        if config.need_add_pool {
            urls.extend(config.pools)
        }
        let sub_url = subconverter.get_clash_sub_url(SubConfig {
            urls,
            config: "config/test.toml".to_string(),
            mixed_port: Some(mixed_port),
            external_url: Some(format!(":{}", external_port)),
            ..Default::default()
        }).await;

        if sub_url.is_empty() {
            error!("当前无可用的待测试订阅连接，请修改配置文件添加订阅链接或确保当前网络通顺");
            subconverter.stop().unwrap();
            return;
        }

        test_sub_file_path = download_test_sub(sub_url).await;
    }

    // 启动 Clash 内核
    let mut clash_meta = ClashMeta::new(external_port, mixed_port);
    if let Err(e) = clash_meta.start().await {
        error!("原神启动失败，第一次启动可能会下载 geo 相关的文件，重新启动即可，打开 logs/clash.log，查看具体错误原因，{}", e);
        clash_meta.stop().unwrap();
        subconverter.stop().unwrap();
        return;
    }

    match clash_meta.get_group(TEST_PROXY_NAME).await {
        Ok(nodes) => {
            info!("开始测试 subs/test/config.yaml 中节点的延迟速度，节点总数：{}", nodes.all.len())
        }
        Err(e) => {
            error!("获取节点数失败，请检查 clash 日志文件和 subs/test/config.yaml 生成的节点是否正确, {}", e);
            clash_meta.stop().unwrap();
            subconverter.stop().unwrap();
            return;
        }
    }

    info!("开始测试连通性");
    let delay_results = test_node_with_delay_config(&clash_meta, &config.connect_test).await;
    let nodes = get_all_tested_nodes(&delay_results);
    info!("连通性测试结果：{} 个节点可用", nodes.len());

    if nodes.is_empty() {
        error!("当前无可用节点，请尝试更换订阅节点或重试");
        clash_meta.stop().unwrap();
        subconverter.stop().unwrap();
        return;
    }

    if config.fast_mode {
        let release_url = subconverter.get_clash_sub_url(SubConfig {
            urls: vec![test_sub_file_path.clone()],
            config: config.sub_config_url,
            includes: Some(nodes),
            ..Default::default()
        }).await;
        let release_sub_file_path = download_release_sub(release_url).await;
        info!("release 文件地址：{}", release_sub_file_path);
        clash_meta.stop().unwrap();
        subconverter.stop().unwrap();
    } else {
        let new_test_sub_url = subconverter.get_clash_sub_url(SubConfig {
            urls: vec![test_sub_file_path.clone()],
            config: "config/test.toml".to_string(),
            includes: Some(nodes),
            mixed_port: Some(clash_meta.mixed_port),
            external_url: Some(format!(":{}", clash_meta.external_port)),
            ..Default::default()
        }).await;

        download_test_sub(new_test_sub_url).await;

        clash_meta.restart().await.unwrap();

        let mut nodes = vec![];
        let mut top_node = String::new();
        for (name, conf) in config.websites {
            info!("当前测试站点：{}, {}", name, conf.url);
            let delay_results = test_node_with_delay_config(&clash_meta, &conf).await;
            if !delay_results.is_empty() {
                nodes = get_all_tested_nodes(&delay_results);
                top_node = get_top_node(&delay_results);
                info!("可用节点数：{}", nodes.len());
                info!("最低延迟节点：{}", top_node);
            }
        }

        let mut node_rename_map: HashMap<String, String> = HashMap::new();
        let mut node_ip_map: HashMap<String, IpAddr> = HashMap::new();
        if nodes.is_empty() {
            error!("当前无可用节点，请尝试更换订阅节点或重试");
            clash_meta.stop().unwrap();
            subconverter.stop().unwrap();
            return;
        }
        for node in &nodes {
            let ip_result = clash_meta.set_group_proxy(TEST_PROXY_NAME, node).await;
            if ip_result.is_ok() {
                let cloudflare_result = cgi_trace::get_ip_by_cloudflare(&clash_meta.proxy_url).await;
                if cloudflare_result.is_ok() {
                    let proxy_ip = cloudflare_result.unwrap();
                    info!("proxy: {}, ip: {}", node, proxy_ip);
                    node_ip_map.insert(node.clone(), proxy_ip);
                } else {
                    error!("获取节点 {} 的 IP 失败, {}", node, cloudflare_result.err().unwrap());
                }
            } else {
                error!("设置节点 {} 失败, {}", node, ip_result.err().unwrap());
            }
        }

        if clash_meta.set_group_proxy(TEST_PROXY_NAME, &top_node).await.is_ok() {
            for (node, ip) in &node_ip_map {
                let ip_detail_result = ip::get_ip_detail_with_proxy(ip, &clash_meta.proxy_url).await;
                match ip_detail_result {
                    Ok(ip_detail) => {
                        info!("{:?}", ip_detail);
                        if config.rename_node {
                            let new_name = config.rename_pattern
                                .replace("${IP}", &ip.to_string())
                                .replace("${COUNTRY_CODE}", &ip_detail.country_code)
                                .replace("${ISP}", &ip_detail.isp)
                                .replace("${CITY}", &ip_detail.city);
                            node_rename_map.insert(node.clone(), new_name);
                        }
                    }
                    Err(e) => {
                        error!("获取节点 {} 的 IP 信息失败, {}", node, e);
                    }
                }
            }
        };


        let release_url = subconverter.get_clash_sub_url(SubConfig {
            urls: vec![test_sub_file_path.clone()],
            config: config.sub_config_url,
            includes: Some(nodes),
            rename: Some(node_rename_map.iter()
                .map(|(k, v)| format!("{}@{}", k, v))
                .collect::<Vec<_>>()),
            ..Default::default()
        }).await;
        info!("release 转换地址：{}", release_url);
        let release_sub_file_path = download_release_sub(release_url).await;
        info!("release 文件地址：{}", release_sub_file_path);
        clash_meta.stop().unwrap();
        subconverter.stop().unwrap();
    }
}

fn get_top_node(test_results: &Vec<HashMap<String, i64>>) -> String {
    let mut combined_data: HashMap<String, Vec<i64>> = HashMap::new();
    for test in test_results {
        for (node, latency) in test {
            combined_data.entry(node.clone()).or_default().push(*latency);
        }
    }
    let node_stats: Vec<(String, i64)> = combined_data.clone()
        .into_iter()
        .map(|(node, latencies)| {
            let sum: i64 = latencies.iter().sum();
            let count = latencies.len() as i64;
            let mean = sum / count;
            (node, mean)
        })
        .collect();
    node_stats.into_iter().min_by_key(|(_, mean)| *mean).unwrap().0
}

async fn test_node_with_delay_config(clash_meta: &ClashMeta, delay_test_config: &DelayTestConfig) -> Vec<HashMap<String, i64>> {
    const ROUND: i32 = 10;
    info!("测试配置：{:?}", delay_test_config);
    let mut delay_results = vec![];

    // 预热 2 轮，DNS lookup
    for _ in 0..2 {
        let _ = clash_meta.test_group(TEST_PROXY_NAME, delay_test_config).await;
    }

    for n in 0..ROUND {
        info!("测试第 {} 轮", n + 1);
        let result = clash_meta.test_group(TEST_PROXY_NAME, delay_test_config).await;

        match result {
            Ok(delay) => {
                delay_results.push(delay.clone());
                info!("有速度节点个数为：{}", delay.len())
            }
            Err(e) => {
                info!("当前测试轮完全没有速度, {}", e)
            }
        }
    }
    delay_results
}

/*
获取所有已测速有过一次速度的节点
 */
fn get_all_tested_nodes(test_results: &Vec<HashMap<String, i64>>) -> Vec<String> {
    let mut keys_set = HashSet::new();
    for result in test_results {
        for key in result.keys() {
            keys_set.insert(key.clone());
        }
    }
    keys_set.into_iter().collect()
}

/*
获取测速稳定的节点
 */
#[allow(dead_code)]
fn get_stable_tested_nodes(test_results: &Vec<HashMap<String, i64>>) -> Vec<String> {
    // 合并所有测试数据
    let mut combined_data: HashMap<String, Vec<i64>> = HashMap::new();
    for test in test_results {
        for (node, latency) in test {
            combined_data.entry(node.clone()).or_default().push(*latency);
        }
    }

    // 计算每个节点的平均延迟和标准差
    let mut node_stats: Vec<(String, f64)> = combined_data.clone()
        .into_iter()
        .filter_map(|(node, latencies)| {
            let sum: i64 = latencies.iter().sum();
            let count = latencies.len();
            if count <= combined_data.len() / 2 {
                None
            } else {
                let mean = sum as f64 / count as f64;
                Some((node, mean))
            }
        })
        .collect();

    // 根据平均延迟对稳定的节点进行排序
    node_stats.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

    node_stats.into_iter().map(|(node, _)| node).collect()
}

async fn download_test_sub(sub_url: String) -> String {
    let client = Client::builder().timeout(Duration::from_secs(120)).build().unwrap();
    let response = client.get(sub_url).send().await.unwrap();
    let content = response.text().await.unwrap();
    let path = "subs/test/config.yaml";
    let mut file = File::create(path).unwrap();
    file.write_all(content.as_bytes()).unwrap();
    env::current_dir().unwrap().join(path).to_string_lossy().to_string()
}

async fn download_release_sub(release_url: String) -> String {
    let client = Client::new();
    let response = client.get(release_url).send().await.unwrap();
    let content = response.text().await.unwrap();
    let now = Local::now();
    let path = format!("subs/release/{}.yaml", now.format("%Y-%m-%d_%H:%M:%S"));
    let mut file = File::create(&path).unwrap();
    file.write_all(content.as_bytes()).unwrap();
    env::current_dir().unwrap().join(path).to_string_lossy().to_string()
}

// 创建目录
fn create_folder() {
    let logs_path = "logs";
    if !Path::new(logs_path).exists() {
        fs::create_dir(logs_path).unwrap()
    }

    let subs_path = "subs";
    if !Path::new(subs_path).exists() {
        fs::create_dir(subs_path).unwrap();
    }

    let test_path = "subs/test";
    if !Path::new(test_path).exists() {
        fs::create_dir(test_path).unwrap();
    }

    let release_path = "subs/release";
    if !Path::new(release_path).exists() {
        fs::create_dir(release_path).unwrap();
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_stable_nodes() {
        // [
        //     { "免费节点2": 829 },
        //     { "免费节点3": 815, "免费节点2": 945, "免费节点1": 838 },
        //     { "免费节点4": 835, "免费节点1": 850, "免费节点3": 819 },
        //     { "免费节点1": 844, "免费节点3": 830, "免费节点2": 856 },
        //     { "免费节点3": 857, "免费节点4": 796, "2": 911, "免费节点4": 816 },
        //     { "免费节点1": 895, "免费节点3": 863, "免费节点4": 829 },
        //     { "免费节点3": 837, "免费节点1": 809, "免费节点4": 849 },
        //     { "免费节点3": 849, "免费节点2": 904, "免费节点4": 892 }
        // ];

        // 假设这是从十组测试中收集的数据
        let test_data = vec![
            HashMap::from([("node1".to_string(), 100), ("node2".to_string(), 200), ("node3".to_string(), 150)]),
            HashMap::from([("node1".to_string(), 110), ("node2".to_string(), 190), ("node3".to_string(), 160)]),
            HashMap::from([("node1".to_string(), 120), ("node3".to_string(), 10000)]),
        ];

        println!("{:?}", get_top_node(&test_data));
    }
}