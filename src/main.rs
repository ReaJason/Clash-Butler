use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::VecDeque;
use std::env;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use clap::Parser;
use proxrs::protocol::Proxy;
use proxrs::sub::SubManager;
use serde_yaml::Mapping;
use serde_yaml::Value as YamlValue;
use tokio::sync::Mutex;
use tracing::error;
use tracing::info;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

use crate::clash::ClashMeta;
use crate::clash::DelayTestConfig;
use crate::settings::Settings;

mod cgi_trace;
mod clash;
mod ip;
mod risk;
mod routes;
mod server;
mod settings;
mod speedtest;
mod website;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    // Starts the Axum server
    #[arg(long)]
    server: bool,
}

const TEST_PROXY_GROUP_NAME: &str = "PROXY";
/// 并发连通性测试 worker 的端口基址，worker i 使用 port_base + i
const WORKER_PORT_BASE: u64 = 8001;

#[tokio::main]
async fn main() {
    tracing::subscriber::set_global_default(
        FmtSubscriber::builder()
            .with_max_level(Level::INFO)
            .finish(),
    )
    .expect("setting default subscriber failed");
    let args = Cli::parse();
    let config = Settings::new();
    match config {
        Ok(config) => {
            // 创建订阅测试所用的目录结构
            create_folder();
            if args.server {
                // 服务端
                // server::start_server(config).await
            } else {
                // 本地生成
                run(config).await
            }
        }
        Err(e) => {
            error!("配置文件读取失败: {}", e)
        }
    }
}

async fn run(config: Settings) {
    let test_yaml_path = "subs/test/config.yaml";
    let test_all_yaml_path = "subs/test/all.yaml";
    let release_yaml_path = env::current_dir().unwrap().join("clash.yaml");
    // let release_base64_path = env::current_dir().unwrap().join("proxies.txt");
    let test_clash_template_path = "conf/clash_test.yaml";
    let release_clash_template_path = "conf/clash_release.yaml";
    let mut urls = config.subs;
    if config.need_add_pool {
        urls.extend(config.pools)
    }
    let test_proxies = SubManager::get_proxies_from_urls(&urls).await;
    info!("待测速节点个数：{}", &test_proxies.len());
    if test_proxies.is_empty() {
        error!("当前无可用的待测试订阅连接，请修改配置文件添加订阅链接或确保当前网络通顺");
        return;
    }

    // 全部保存一下节点信息
    SubManager::save_proxies_into_clash_file(
        &test_proxies,
        test_clash_template_path.to_string(),
        test_all_yaml_path.to_string(),
    );

    let chunk_size = config.test_group_size;
    let proxies_group: Vec<_> = test_proxies
        .chunks(chunk_size)
        .map(|p| p.to_vec())
        .collect();
    let group_size = proxies_group.len();
    if group_size > 1 {
        info!(
            "为加速测试速度，以 {} 为限制分为 {} 组测试",
            chunk_size,
            proxies_group.len()
        );
    }

    // 启动 Clash 内核
    let external_port = 9095;
    let mixed_port = 7998;
    let mut useful_proxies = Vec::new();
    for (index, proxies) in proxies_group.iter().enumerate() {
        if group_size > 1 {
            info!("正在测试第 {} 组", index + 1)
        }

        SubManager::save_proxies_into_clash_file(
            proxies,
            test_clash_template_path.to_string(),
            test_yaml_path.to_string(),
        );

        let mut clash_meta = ClashMeta::new(external_port, mixed_port);
        if let Err(e) = clash_meta.start().await {
            error!("原神启动失败，第一次启动可能会下载 geo 相关的文件，重新启动即可，打开 logs/clash.log，查看具体错误原因，{}", e);
            clash_meta.stop().unwrap();
            continue;
        }

        match clash_meta.get_group(TEST_PROXY_GROUP_NAME).await {
            Ok(nodes) => {
                info!(
                    "开始测试 subs/test/config.yaml 中节点的延迟速度，节点总数：{}",
                    nodes.all.len()
                )
            }
            Err(e) => {
                error!("获取节点数失败，请检查 clash 日志文件和 subs/test/config.yaml 生成的节点是否正确, {}", e);
                clash_meta.stop().unwrap();
                continue;
            }
        }

        info!("开始测试连通性");
        let delay_results = test_node_with_delay_config(&clash_meta, &config.connect_test).await;
        let nodes = get_all_tested_nodes(&delay_results);
        info!("连通性测试结果：{} 个节点可用", nodes.len());
        if !nodes.is_empty() {
            let cur_useful_proxies = proxies
                .iter()
                .filter(|&proxy| nodes.contains(&proxy.get_name().to_string()))
                .cloned()
                .collect::<Vec<Proxy>>();
            info!("cur_useful_proxies len: {}", &cur_useful_proxies.len());
            useful_proxies.extend(cur_useful_proxies);
            info!("useful_proxies len: {}", useful_proxies.len());
        }
        clash_meta.stop().unwrap();
    }

    if useful_proxies.is_empty() {
        error!("当前无可用节点，请尝试更换订阅节点或重试");
        return;
    } else {
        info!("当前总可用节点个数：{}", &useful_proxies.len());
    }
    let timeout: Duration = Duration::from_millis(config.connect_test.timeout + 2000);
    if config.fast_mode {
        SubManager::save_proxies_into_clash_file(
            &useful_proxies,
            release_clash_template_path.to_string(),
            release_yaml_path.to_string_lossy().to_string(),
        );
        info!("release 文件地址：{}", release_yaml_path.to_string_lossy());
    } else {
        let mut clash_meta = ClashMeta::new(external_port, mixed_port);
        SubManager::save_proxies_into_clash_file(
            &useful_proxies,
            test_clash_template_path.to_string(),
            test_yaml_path.to_string(),
        );

        let nodes = &mut useful_proxies
            .iter()
            .map(|p| p.get_name().to_string())
            .collect::<Vec<String>>();
        let mut node_rename_map: HashMap<String, String> = HashMap::new();
        if config.rename_node {
            if nodes.is_empty() {
                error!("当前无可用节点，请尝试更换订阅节点或重试");
                return;
            }
            // 注入独立的 select 组和监听端口，让 worker 并发切组互不影响
            if let Err(e) = inject_worker_groups(
                test_yaml_path,
                nodes,
                config.rename_test_workers,
                WORKER_PORT_BASE,
            ) {
                error!("注入并发测试配置失败，{}", e);
                return;
            }
        }

        if let Err(e) = clash_meta.start().await {
            error!("原神启动失败，第一次启动可能会下载 geo 相关的文件，重新启动即可，打开 logs/clash.log，查看具体错误原因，{}", e);
            clash_meta.stop().unwrap();
            return;
        }
        info!("当前节点个数为：{}", useful_proxies.len());

        if config.rename_node {
            let shared_meta = Arc::new(clash_meta);
            let (alive_nodes, rename_map) = test_nodes_concurrently(
                &shared_meta,
                nodes,
                config.rename_test_workers,
                WORKER_PORT_BASE,
                timeout,
                config.rename_pattern.clone(),
            )
            .await;
            *nodes = alive_nodes;
            node_rename_map = rename_map;
            info!("网站连通性测试结果：{} 个节点可用", nodes.len());
            clash_meta =
                Arc::try_unwrap(shared_meta).unwrap_or_else(|_| panic!("仍有 worker 未退出"));
        }

        let mut release_proxies = useful_proxies
            .into_iter()
            .filter(|proxy: &Proxy| nodes.contains(&proxy.get_name().to_string()))
            .collect::<Vec<Proxy>>();

        if !node_rename_map.is_empty() {
            for proxy in &mut release_proxies {
                let name = if let Some(new_name) = node_rename_map.get(proxy.get_name()) {
                    new_name.clone()
                } else {
                    proxy.get_name().to_string()
                };
                proxy.set_name(&name);
            }
        }

        SubManager::rename_dup_proxies_name(&mut release_proxies);
        SubManager::save_proxies_into_clash_file(
            &release_proxies,
            release_clash_template_path.to_string(),
            release_yaml_path.to_string_lossy().to_string(),
        );
        info!("release 文件地址：{}", release_yaml_path.to_string_lossy());
        clash_meta.stop().unwrap();
    }
}

/// 向测试配置注入 N 个独立的 select 组和绑定端口的 listener，
/// 使网站连通性测试可以并发进行（每个 worker 独占一组一端口，切组互不影响）
fn inject_worker_groups(
    config_path: &str,
    proxy_names: &[String],
    worker_count: usize,
    port_base: u64,
) -> std::io::Result<()> {
    let worker_count = worker_count.min(proxy_names.len()).max(1);
    let content = fs::read_to_string(config_path)?;
    let mut yaml: YamlValue = serde_yaml::from_str(&content).expect("解析测试配置失败");

    let groups = yaml
        .get_mut("proxy-groups")
        .and_then(YamlValue::as_sequence_mut)
        .expect("测试配置缺少 proxy-groups");
    for w in 0..worker_count {
        let mut group = Mapping::new();
        group.insert(
            YamlValue::String("name".to_string()),
            YamlValue::String(format!("TEST{w}")),
        );
        group.insert(
            YamlValue::String("type".to_string()),
            YamlValue::String("select".to_string()),
        );
        group.insert(
            YamlValue::String("proxies".to_string()),
            YamlValue::Sequence(
                proxy_names
                    .iter()
                    .map(|n| YamlValue::String(n.clone()))
                    .collect(),
            ),
        );
        groups.push(YamlValue::Mapping(group));
    }

    let listeners: Vec<YamlValue> = (0..worker_count)
        .map(|w| {
            let mut listener = Mapping::new();
            listener.insert(
                YamlValue::String("name".to_string()),
                YamlValue::String(format!("worker{w}")),
            );
            listener.insert(
                YamlValue::String("type".to_string()),
                YamlValue::String("mixed".to_string()),
            );
            listener.insert(
                YamlValue::String("port".to_string()),
                YamlValue::Number((port_base + w as u64).into()),
            );
            listener.insert(
                YamlValue::String("proxy".to_string()),
                YamlValue::String(format!("TEST{w}")),
            );
            YamlValue::Mapping(listener)
        })
        .collect();
    yaml.as_mapping_mut()
        .expect("测试配置不是 mapping")
        .insert(
            YamlValue::String("listeners".to_string()),
            YamlValue::Sequence(listeners),
        );

    fs::write(config_path, serde_yaml::to_string(&yaml).expect("序列化测试配置失败"))
}

enum NodeOutcome {
    /// 节点保留；new_name 为 None 表示未参与测试（切组失败），不重命名
    Alive { node: String, new_name: Option<String> },
    /// 节点不可用，过滤
    Dead,
}

/// 测试单个节点：获取出口 IP、并发检测 gemini/claude 连通性，并生成重命名
async fn test_node(
    clash_meta: &ClashMeta,
    group: &str,
    proxy_url: &str,
    node: String,
    timeout: Duration,
    rename_pattern: &str,
) -> NodeOutcome {
    if let Err(e) = clash_meta.set_group_proxy(group, &node).await {
        error!("设置节点 {} 失败，{}", node, e);
        return NodeOutcome::Alive {
            node,
            new_name: None,
        };
    }
    let (proxy_ip, from) = match cgi_trace::get_ip(proxy_url, timeout).await {
        Ok(result) => result,
        Err(e) => {
            error!("获取节点 {} 的 IP 失败，{}", node, e);
            return NodeOutcome::Dead;
        }
    };
    info!("[{}] ip: {} from: {}", node, proxy_ip, from);

    let (gemini_result, claude_result) = tokio::join!(
        website::gemini_is_ok(proxy_url, timeout),
        website::claude_is_ok(proxy_url, timeout),
    );
    let gemini_is_ok = match gemini_result {
        Ok(_) => {
            info!("[{}] gemini is ok", node);
            true
        }
        Err(err) => {
            error!("[{}] gemini is not ok, {:#}", node, err);
            false
        }
    };
    let claude_is_ok = match claude_result {
        Ok(_) => {
            info!("[{}] claude is ok", node);
            true
        }
        Err(err) => {
            error!("[{}] claude is not ok, {:#}", node, err);
            false
        }
    };
    if !gemini_is_ok && !claude_is_ok {
        return NodeOutcome::Dead;
    }

    let mut new_name = proxy_ip.to_string();
    match ip::get_ip_detail(&proxy_ip, proxy_url).await {
        Ok(ip_detail) => {
            info!("{:?}", ip_detail);
            new_name = rename_pattern
                .replace("${IP}", &proxy_ip.to_string())
                .replace("${COUNTRYCODE}", &ip_detail.country_code)
                .replace("${ISP}", &ip_detail.isp)
                .replace("${CITY}", &ip_detail.city);
        }
        Err(e) => {
            error!("获取节点 {node} 的 IP 信息失败，{e}");
        }
    }
    if gemini_is_ok {
        new_name += "_Gemini";
    }
    if claude_is_ok {
        new_name += "_Claude";
    }
    NodeOutcome::Alive {
        node,
        new_name: Some(new_name),
    }
}

/// 并发测试所有节点的网站连通性与出口 IP，返回存活节点名与重命名映射
async fn test_nodes_concurrently(
    clash_meta: &Arc<ClashMeta>,
    nodes: &[String],
    worker_count: usize,
    port_base: u64,
    timeout: Duration,
    rename_pattern: String,
) -> (Vec<String>, HashMap<String, String>) {
    let worker_count = worker_count.min(nodes.len()).max(1);
    info!("并发测试节点网站连通性，worker 数：{}", worker_count);
    let queue = Arc::new(Mutex::new(VecDeque::from(nodes.to_vec())));
    let mut handles = Vec::with_capacity(worker_count);
    for w in 0..worker_count {
        let group = format!("TEST{w}");
        let proxy_url = format!("http://127.0.0.1:{}", port_base + w as u64);
        let queue = Arc::clone(&queue);
        let clash_meta = Arc::clone(clash_meta);
        let rename_pattern = rename_pattern.clone();
        handles.push(tokio::spawn(async move {
            let mut outcomes = Vec::new();
            loop {
                // 锁只在取节点时持有，测试期间释放
                let node = queue.lock().await.pop_front();
                let Some(node) = node else { break };
                outcomes.push(
                    test_node(&clash_meta, &group, &proxy_url, node, timeout, &rename_pattern)
                        .await,
                );
            }
            outcomes
        }));
    }

    let mut alive_nodes = Vec::new();
    let mut node_rename_map = HashMap::new();
    for handle in handles {
        for outcome in handle.await.expect("worker 任务异常退出") {
            match outcome {
                NodeOutcome::Alive { node, new_name } => {
                    if let Some(new_name) = new_name {
                        node_rename_map.insert(node.clone(), new_name);
                    }
                    alive_nodes.push(node);
                }
                NodeOutcome::Dead => {}
            }
        }
    }
    (alive_nodes, node_rename_map)
}

#[allow(dead_code)]
fn get_top_node(test_results: &Vec<HashMap<String, i64>>) -> (String, i64) {
    let mut combined_data: HashMap<String, Vec<i64>> = HashMap::new();
    for test in test_results {
        for (node, latency) in test {
            combined_data
                .entry(node.clone())
                .or_default()
                .push(*latency);
        }
    }
    let node_stats: Vec<(String, i64)> = combined_data
        .clone()
        .into_iter()
        .map(|(node, latencies)| {
            let sum: i64 = latencies.iter().sum();
            let count = latencies.len() as i64;
            let mean = sum / count;
            (node, mean)
        })
        .collect();
    node_stats
        .into_iter()
        .min_by_key(|(_, mean)| *mean)
        .unwrap()
}

async fn test_node_with_delay_config(
    clash_meta: &ClashMeta,
    delay_test_config: &DelayTestConfig,
) -> Vec<HashMap<String, i64>> {
    const ROUND: i32 = 5;
    info!("测试配置：{:?}", delay_test_config);
    let mut delay_results = vec![];

    // 预热 2 轮，DNS lookup
    for _ in 0..2 {
        let _ = clash_meta
            .test_group(TEST_PROXY_GROUP_NAME, delay_test_config)
            .await;
    }

    for n in 0..ROUND {
        info!("测试第 {} 轮", n + 1);
        let result = clash_meta
            .test_group(TEST_PROXY_GROUP_NAME, delay_test_config)
            .await;

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
            combined_data
                .entry(node.clone())
                .or_default()
                .push(*latency);
        }
    }

    // 计算每个节点的平均延迟和标准差
    let mut node_stats: Vec<(String, f64)> = combined_data
        .clone()
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
    fn test_inject_worker_groups() {
        let dir = env::temp_dir().join("clash-butler-test");
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.yaml");
        fs::write(
            &path,
            "mixed-port: 7998\nproxies: []\nproxy-groups:\n  - name: PROXY\n    type: select\n    proxies: []\nrules:\n  - MATCH,PROXY\n",
        )
        .unwrap();
        inject_worker_groups(
            path.to_str().unwrap(),
            &["节点A".to_string(), "节点B".to_string()],
            2,
            WORKER_PORT_BASE,
        )
        .unwrap();
        let content = fs::read_to_string(&path).unwrap();
        println!("{}", content);
        let yaml: YamlValue = serde_yaml::from_str(&content).unwrap();
        let groups = yaml["proxy-groups"].as_sequence().unwrap();
        assert_eq!(groups.len(), 3);
        assert_eq!(groups[1]["name"].as_str().unwrap(), "TEST0");
        assert_eq!(
            groups[2]["proxies"].as_sequence().unwrap().len(),
            2
        );
        let listeners = yaml["listeners"].as_sequence().unwrap();
        assert_eq!(listeners.len(), 2);
        assert_eq!(listeners[0]["port"].as_u64().unwrap(), 8001);
        assert_eq!(listeners[1]["proxy"].as_str().unwrap(), "TEST1");
    }

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
            HashMap::from([
                ("node1".to_string(), 100),
                ("node2".to_string(), 200),
                ("node3".to_string(), 150),
            ]),
            HashMap::from([
                ("node1".to_string(), 110),
                ("node2".to_string(), 190),
                ("node3".to_string(), 160),
            ]),
            HashMap::from([("node1".to_string(), 120), ("node3".to_string(), 10000)]),
        ];

        println!("{:?}", get_top_node(&test_data));
    }

    #[test]
    fn test_rename_pattern() {
        let count = "${COUNTRYCODE}_${CITY}_${ISP}".matches('_').count();
        println!("{count}");
        let count = "HongKong_Jordan_VertexConnectivityLLC62"
            .matches('_')
            .count();
        println!("{count}")
    }
}
