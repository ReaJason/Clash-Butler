use std::collections::HashMap;
use std::collections::HashSet;
use std::fs;
use std::fs::File;
use std::hash::DefaultHasher;
use std::hash::Hash;
use std::hash::Hasher;
use std::io;
use std::io::Read;
use std::io::Write;
use std::path::Path;
use std::time::Duration;

use regex::Regex;
use reqwest::Client;
use serde_yaml::Mapping;
use serde_yaml::Value;
use tokio::time::sleep;
use tracing::info;

use crate::base64::base64decode;
use crate::protocol::Proxy;

#[derive(Debug)]
pub struct SubManager {}

impl SubManager {
    /// 从链接中获取代理信息支持以下四种结构
    /// 1. http://订阅链接，传入代理地址
    /// 2. C:\\文件地址 /home/yaml，传入文件地址
    /// 3. ss://xxxx，传入单个节点链接
    /// 4. edhxxx, 传入 base64 的节点信息
    pub async fn get_proxies_from_url(url: String) -> Vec<Proxy> {
        let mut proxies: Vec<Proxy> = Vec::new();
        if url.starts_with("http") {
            if let Ok(file_path) = Self::get_content_from_sub_url(&url).await {
                proxies = Self::parse_content(file_path).unwrap();
            }
        } else if Path::new(&url).is_file() {
            proxies = Self::parse_from_path(&url).unwrap();
        } else if let Ok(p) = Self::parse_content(url.to_string()) {
            proxies.extend(p);
        }
        info!("{} parsed proxies: {}", &url, &proxies.len());
        proxies
    }

    /// 传入 urls 列表解析代理
    pub async fn get_proxies_from_urls(subs: &Vec<String>) -> Vec<Proxy> {
        let mut proxies: Vec<Proxy> = Vec::new();
        for url in subs {
            proxies.extend(Self::get_proxies_from_url(url.to_string()).await)
        }

        if !proxies.is_empty() {
            proxies = Self::exclude_dup_proxies(proxies);
            Self::rename_dup_proxies_name(&mut proxies);
        }

        proxies
    }

    async fn get_content_from_sub_url(sub_url: &str) -> Result<String, Box<dyn std::error::Error>> {
        let client = Client::new();
        let mut attempts = 0;
        let retries = 3;

        loop {
            let result = client
                .get(sub_url)
                .timeout(Duration::from_secs(10))
                .send()
                .await;
            match result {
                Ok(resp) => {
                    let status = resp.status();
                    return if status.is_success() {
                        // 获取 UUID 作为文件名
                        // let re = Regex::new(r"files/(.*?)/raw").unwrap();
                        // let uuid = re.captures(sub_url)
                        //     .and_then(|caps| caps.get(1))
                        //     .map_or_else(|| {
                        //         format!("{:x}", md5::compute(sub_url))
                        //     }, |m| m.as_str().to_string());

                        // let file_path = PathBuf::from_iter(vec!["subs", &uuid.to_string()]);
                        // let mut file = File::create(&file_path).unwrap();

                        let content_result = resp.text().await;
                        match content_result {
                            Ok(content) => {
                                // file.write_all(content.as_bytes()).unwrap();
                                // Ok(env::current_dir().unwrap().join(file_path).to_string_lossy().
                                // to_string())
                                Ok(content)
                            }
                            Err(e) => {
                                if e.is_timeout() {
                                    continue;
                                }
                                return Err(Box::new(e));
                            }
                        }
                    } else {
                        Err(format!("获取订阅连失败 {} 响应码 {}", sub_url, status.as_str()).into())
                    };
                }
                Err(e) => {
                    if !e.is_timeout() {
                        return Err(Box::new(e));
                    }
                }
            }

            if attempts < retries {
                attempts += 1;
                sleep(Duration::from_secs(1)).await;
            } else {
                return Err(format!(
                    "当前链接 {} 无法访问，已跳过，或请确保当前网络通顺",
                    sub_url
                )
                .into());
            }
        }
    }

    /// 从本地文件中解析代理
    pub fn parse_from_path<P: AsRef<Path>>(
        file_path: P,
    ) -> Result<Vec<Proxy>, Box<dyn std::error::Error>> {
        match fs::read_to_string(file_path) {
            Ok(contents) => Ok(Self::parse_content(contents)?),
            Err(e) => Err(format!("Error reading file: {}", e).into()),
        }
    }

    /// 从字符串中解析代理
    /// 1. 先尝试使用 yaml 格式解析
    /// 2. 尝试解析 base64 格式
    /// 3. 尝试使用纯链接格式解析
    pub fn parse_content(content: String) -> Result<Vec<Proxy>, Box<dyn std::error::Error>> {
        let conf_proxies: Vec<Proxy> = Vec::new();
        match Self::parse_yaml_content(&content) {
            Ok(proxies) => return Ok(proxies),
            Err(_) => match Self::parse_base64_content(&content) {
                Ok(proxies) => return Ok(proxies),
                Err(_) => {
                    if let Ok(proxies) = Self::parse_links_content(&content) {
                        return Ok(proxies);
                    }
                }
            },
        }
        Ok(conf_proxies)
    }

    fn parse_yaml_content(content: &str) -> Result<Vec<Proxy>, Box<dyn std::error::Error>> {
        let mut conf_proxies: Vec<Proxy> = Vec::new();
        let yaml = serde_yaml::from_str::<serde_json::Value>(content)?;
        match yaml.get("proxies").or_else(|| yaml.get("Proxies")) {
            None => {
                return Err(format!("Proxy not found: {}", content).into());
            }
            Some(proxies) => {
                if let Some(proxies_arr) = proxies.as_array() {
                    for proxy in proxies_arr {
                        let result = Proxy::from_json(&proxy.to_string());
                        match result {
                            Ok(p) => {
                                conf_proxies.push(p);
                            }
                            Err(e) => {
                                println!("{} {:?}", e, proxy);
                            }
                        }
                    }
                }
            }
        }
        Ok(conf_proxies)
    }

    fn parse_base64_content(content: &str) -> Result<Vec<Proxy>, Box<dyn std::error::Error>> {
        let mut conf_proxies: Vec<Proxy> = Vec::new();
        let base64 = base64decode(content.trim());
        base64
            .split("\n")
            .filter(|line| !line.is_empty())
            .for_each(|line| match Proxy::from_link(line.trim().to_string()) {
                Ok(proxy) => conf_proxies.push(proxy),
                Err(e) => {
                    println!("{}", e);
                }
            });
        Ok(conf_proxies)
    }

    fn parse_links_content(content: &str) -> Result<Vec<Proxy>, Box<dyn std::error::Error>> {
        let mut conf_proxies: Vec<Proxy> = Vec::new();
        let links = content
            .split("\n")
            .filter(|line| !line.is_empty())
            .map(|link| link.trim())
            .collect::<Vec<&str>>();
        for link in links {
            if let Ok(proxy) = Proxy::from_link(link.trim().to_string()) {
                conf_proxies.push(proxy)
            }
        }
        Ok(conf_proxies)
    }

    /// 移除重复节点
    pub fn exclude_dup_proxies(proxies: Vec<Proxy>) -> Vec<Proxy> {
        let mut new_proxies = Vec::new();
        if !proxies.is_empty() {
            let set: HashSet<Proxy> = HashSet::from_iter(proxies);
            new_proxies = set.into_iter().collect();
            new_proxies.sort_by(|a, b| a.proxy_type.cmp(&b.proxy_type));
        }
        new_proxies
    }

    /// 重置节点名称
    #[allow(dead_code)]
    pub fn unset_proxies_name(proxies: &mut Vec<Proxy>) {
        for proxy in proxies {
            let server = proxy.get_server().to_string();
            let hash = &mut DefaultHasher::new();
            proxy.to_json().unwrap().hash(hash);
            let h = hash.finish();
            proxy.set_name(&(server + "_" + &h.to_string()[..5]));
        }
    }

    /// 重命名相同名称的节点，在末尾加序号
    pub fn rename_dup_proxies_name(proxies: &mut Vec<Proxy>) {
        let mut name_counts: HashMap<String, usize> = HashMap::new();
        let number_suffix = Regex::new(r"\d+$").unwrap();

        // 打点，并删除其中原有的数字后缀
        for proxy in proxies.iter_mut() {
            let mut name = proxy.get_name().to_string();
            name = number_suffix.replace(&name, "").to_string();
            proxy.set_name(&name);
            *name_counts.entry(name).or_insert(0) += 1;
        }

        for proxy in &mut *proxies {
            let name = proxy.get_name().to_string();
            if let Some(count) = name_counts.get(&name) {
                if count > &1 {
                    let mut counter = 1;
                    let mut new_name = format!("{}{}", &name, counter);
                    while name_counts.contains_key(&new_name) {
                        counter += 1;
                        new_name = format!("{}{}", &name, counter);
                    }

                    proxy.set_name(&new_name);
                    name_counts.insert(new_name, 1);
                }
            }
        }

        // 以名称重新排序
        proxies.sort_by(|a, b| a.get_name().cmp(b.get_name()));
    }

    // 通过配置格式，获取 clash 配置文件内容
    pub fn get_clash_config_content(
        config_path: String,
        new_proxies: &Vec<Proxy>,
    ) -> io::Result<String> {
        let mut file = File::open(config_path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        let mut yaml: Value = serde_yaml::from_str(&contents).expect("Failed to parse YAML");

        // 插入 proxies
        if let Some(proxies) = yaml.get_mut("proxies").and_then(Value::as_sequence_mut) {
            for proxy in new_proxies {
                proxies.push(Value::Mapping(
                    serde_yaml::from_str::<Mapping>(&proxy.to_json()?).unwrap(),
                ));
            }
        } else {
            println!("Failed to find 'proxies' in the YAML file");
        }

        // 处理 proxy-groups 逻辑
        if let Some(groups) = yaml
            .get_mut("proxy-groups")
            .and_then(Value::as_sequence_mut)
        {
            for group in groups.iter_mut() {
                if let Some(group_map) = group.as_mapping_mut() {
                    if let Some(Value::String(filter)) =
                        group_map.get(Value::String("filter".to_string()))
                    {
                        let regex = Regex::new(filter).expect("Invalid regex");
                        if let Some(proxies) = group_map
                            .get_mut(Value::String("proxies".to_string()))
                            .and_then(Value::as_sequence_mut)
                        {
                            let mut removed_default = false;
                            for proxy in new_proxies {
                                if regex.is_match(proxy.get_name()) {
                                    if !removed_default
                                        && proxies
                                            .first()
                                            .is_some_and(|p| p.as_str().unwrap().eq("PROXY"))
                                    {
                                        proxies.remove(0);
                                        removed_default = true;
                                    }
                                    proxies.push(Value::String(proxy.get_name().to_string()));
                                }
                            }
                            if proxies.is_empty() {
                                proxies.push(Value::String("DIRECT".to_string()));
                            }
                        }
                    }
                }
            }
        }
        Ok(serde_yaml::to_string(&yaml).expect("Failed to serialize YAML"))
    }

    pub fn save_proxies_into_clash_file(
        proxies: &Vec<Proxy>,
        config_path: String,
        save_path: String,
    ) {
        let content = SubManager::get_clash_config_content(config_path, proxies).unwrap();
        let mut file = File::create(&save_path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
    }

    pub fn save_proxies_into_base64_file(proxies: &[Proxy], save_path: String) {
        let mut file = File::create(&save_path).unwrap();
        let content = proxies
            .iter()
            .map(|p| p.adapter.to_link())
            .collect::<Vec<_>>();
        for b in content {
            writeln!(file, "{}", b).unwrap();
        }
    }
}

#[cfg(test)]
mod test {
    use std::path::PathBuf;

    use super::*;
    use crate::protocol;
    use crate::protocol::ProxyType::Hysteria2;
    use crate::protocol::ProxyType::Vless;
    use crate::protocol::ProxyType::Vmess;

    #[test]
    fn test_get_clash_config_content() {
        let path = "conf/clash_release.yaml";
        let mut proxies = SubManager::parse_from_path(
            "/Users/reajason/RustroverProjects/clash-butler/subs/0c1149d13476bbe3b62eecb7c9b895f4",
        )
        .unwrap();
        SubManager::unset_proxies_name(&mut proxies);
        let content = SubManager::get_clash_config_content(path.to_string(), &proxies).unwrap();
        println!("{}", content);
    }

    #[test]
    fn test_urls_type() {
        let link = "ss://YWVzLTEyOC1nY206ZDljNTc3MzI4ZmIzNDlmZQ==@120.232.73.68:40676#%F0%9F%87%AD%F0%9F%87%B0HK";
        assert!(!Path::new(link).is_file());

        let path = PathBuf::from_iter(vec!["tests", "res", "base64_proxies"]);
        assert!(path.is_file());
    }

    #[tokio::test]
    #[ignore]
    async fn test_parse_conf() {
        let url = "https://github.com/ripaojiedian/freenode/raw/refs/heads/main/sub".to_string();
        let proxies = SubManager::get_proxies_from_url(url).await;
        for proxy in &proxies {
            println!("{:?}", proxy);
        }
    }

    #[test]
    fn test_regex_filter() {
        let filter = "台湾|TW|Tw|Taiwan|新北|彰化|CHT|HINET";
        let name = "JP_Tokyo_Shenzhen lesuyun Network Technology";
        let is_match = Regex::new(filter).unwrap().is_match(name);
        assert!(!is_match);
    }

    #[test]
    fn test_rename_dup_proxies_name() {
        let content = String::from(
            "ss://cmM0LW1kNToydnpobzU=@120.241.144.101:2410#name\n\
        ss://cmM0LW1kNToydnpobzU=@120.241.144.101:2410#name1\n\
        ss://cmM0LW1kNToydnpobzU=@120.241.144.101:2410#name1\n\
        ss://cmM0LW1kNToydnpobzU=@120.241.144.101:2410#name\n\
        ss://cmM0LW1kNToydnpobzU=@120.241.144.101:2410#xixi",
        );

        let mut proxies = SubManager::parse_content(content).unwrap();
        assert_eq!(proxies.len(), 5);
        assert_eq!(proxies.get(0).unwrap().get_name(), "name");
        assert_eq!(proxies.get(1).unwrap().get_name(), "name1");
        assert_eq!(proxies.get(2).unwrap().get_name(), "name1");
        assert_eq!(proxies.get(3).unwrap().get_name(), "name");
        assert_eq!(proxies.get(4).unwrap().get_name(), "xixi");
        SubManager::rename_dup_proxies_name(&mut proxies);
        assert_eq!(proxies.len(), 5);
        assert_eq!(proxies.get(0).unwrap().get_name(), "name1");
        assert_eq!(proxies.get(1).unwrap().get_name(), "name2");
        assert_eq!(proxies.get(2).unwrap().get_name(), "name3");
        assert_eq!(proxies.get(3).unwrap().get_name(), "name4");
        assert_eq!(proxies.get(4).unwrap().get_name(), "xixi");
    }

    #[tokio::test]
    async fn test_merge_config() {
        let urls = vec![
            "hysteria2://bc97f674-c578-4940-9234-0a1da46041b9@188.68.234.53:36604/?sni=www.bing.com&alpn=h3&insecure=1#s14panel".to_string(),
            "hysteria2://bc97f674-c578-4940-9234-0a1da46041b9@188.68.248.8:8220?peer=www.bing.com&insecure=1&alpn=h3#s15panel".to_string(),
            "ss://Y2hhY2hhMjA6c0pNZGNJN05QakAxNC4xOC4yNTMuMTc4OjkwMDU#175.29.122.147_OpenAI".to_string(),
            "ss://Y2hhY2hhMjA6djVhVVV0bWUzanhzQDE0LjE4LjI1My4xNzg6OTAwMw#175.29.122.149_OpenAI_Claude".to_string(),
            "vmess://YXV0bzo5MjA0YWZjZC0wMjNlLTc4MWYtMWFiYy1jMTJlZmNjZDEzNDRAMTgzLjIzMi4xOTcuMjIzOjMzODAz?remarks=Tokyo-Akamai-H&path=/ray&obfs=websocket&tls=1&alterId=0".to_string(),
            "vmess://YXV0bzo5MjA0YWZjZC0wMjNlLTc4MWYtMWFiYy1jMTJlZmNjZDEzNDRAMTEzLjU2LjIxOC4xMzozMzgwMA?remarks=Tokyo-Akamai-H&path=/ray&obfs=websocket&tls=1&alterId=0".to_string(),
            "vmess://YXV0bzo5MjA0YWZjZC0wMjNlLTc4MWYtMWFiYy1jMTJlZmNjZDEzNDRAMTIyLjE5NS4xODkuMTI0OjMzODAw?remarks=Tokyo-Akamai-H&path=/ray&obfs=websocket&tls=1&alterId=0".to_string(),
            "vmess://YXV0bzo5MjA0YWZjZC0wMjNlLTc4MWYtMWFiYy1jMTJlZmNjZDEzNDRANDMuMjQ4LjExOS4xNDU6MzM0MDc?remarks=%E9%A6%99%E6%B8%AF%E9%98%BF%E9%87%8C%E4%BA%91-H&path=/ray&obfs=websocket&tls=1&alterId=0".to_string(),
        ];
        let proxies = SubManager::get_proxies_from_urls(&urls).await;
        let release_clash_template_path =
            "/Users/reajason/RustroverProjects/clash-butler/conf/clash_release.yaml".to_string();
        let save_path =
            "/Users/reajason/RustroverProjects/clash-butler/subs/release/proxy-merge.yaml"
                .to_string();
        SubManager::save_proxies_into_clash_file(&proxies, release_clash_template_path, save_path);
    }

    #[tokio::test]
    async fn test_rename() {
        let urls = vec!["/Users/reajason/RustroverProjects/clash-butler/clash.yaml".to_string()];
        let mut proxies = SubManager::get_proxies_from_urls(&urls).await;
        SubManager::rename_dup_proxies_name(&mut proxies);
        let release_clash_template_path =
            "/Users/reajason/RustroverProjects/clash-butler/conf/clash_release.yaml".to_string();
        let save_path = "/Users/reajason/RustroverProjects/clash-butler/clash1.yaml".to_string();
        SubManager::save_proxies_into_clash_file(&proxies, release_clash_template_path, save_path)
    }

    #[tokio::test]
    async fn test_merge_uuids() {
        let url = "/Users/reajason/.config/clash.meta/template4.yaml";
        let mut proxies = SubManager::get_proxies_from_url(url.to_string()).await;

        let mut result = vec![];
        let uuids = vec![
            "f425df23-6ab6-449a-87bd-3ba74fdc1777",
            "742104bc-2d31-4139-9db1-848e36713207",
            "839ebb68-8a8b-4cf3-aa78-bf8d8721cd04",
        ];

        for uuid in uuids {
            for proxy in &mut proxies {
                println!("{:?}", proxy);
                if proxy.proxy_type.eq(&Vless) {
                    if let Some(vless) = proxy
                        .adapter
                        .as_any()
                        .downcast_ref::<protocol::vless::Vless>()
                    {
                        let mut p = vless.clone();
                        p.uuid = uuid.to_string();
                        proxy.adapter = Box::new(p);
                        result.push(proxy.clone());
                    } else {
                    }
                } else if proxy.proxy_type.eq(&Vmess) {
                    if let Some(vmess) = proxy
                        .adapter
                        .as_any()
                        .downcast_ref::<protocol::vmess::Vmess>()
                    {
                        let mut p = vmess.clone();
                        p.uuid = uuid.to_string();
                        proxy.adapter = Box::new(p);
                        result.push(proxy.clone());
                    }
                } else if proxy.proxy_type.eq(&Hysteria2) {
                    if let Some(hysteria2) = proxy
                        .adapter
                        .as_any()
                        .downcast_ref::<protocol::hysteria2::Hysteria2>()
                    {
                        let mut p = hysteria2.clone();
                        p.password = uuid.to_string();
                        proxy.adapter = Box::new(p);
                        result.push(proxy.clone());
                    }
                }
            }
        }

        SubManager::rename_dup_proxies_name(&mut result);

        SubManager::save_proxies_into_clash_file(
            &result,
            "/Users/reajason/RustroverProjects/clash-butler/conf/clash_release.yaml".to_string(),
            "/Users/reajason/RustroverProjects/clash-butler/2025.02.17.yaml".to_string(),
        );

        println!("{:?}", result.len());
    }
}
