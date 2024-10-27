use crate::base64::base64decode;
use crate::protocol::Proxy;
use regex::Regex;
use reqwest::Client;
use serde_yaml::{Mapping, Value};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::io::{Read, Write};
use std::path::Path;
use std::time::Duration;
use std::{fs, io};
use tokio::time::sleep;

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
            match Self::get_content_from_sub_url(&url).await {
                Ok(file_path) => {
                    proxies = Self::parse_content(file_path).unwrap();
                }
                Err(_) => {

                }
            }
        } else {
            if Path::new(&url).is_file() {
                proxies = Self::parse_from_path(&url).unwrap();
            } else {
                match Self::parse_content(url.to_string()) {
                    Ok(p) => {
                        proxies.extend(p);
                    }
                    Err(_) => {
                    }
                }
            }
        }
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
                                // Ok(env::current_dir().unwrap().join(file_path).to_string_lossy().to_string())
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
                return Err(format!("当前链接 {} 无法访问，已跳过，或请确保当前网络通顺", sub_url).into());
            }
        }
    }

    /// 从本地文件中解析代理
    pub fn parse_from_path<P: AsRef<Path>>(file_path: P) -> Result<Vec<Proxy>, Box<dyn std::error::Error>> {
        let conf_proxies;
        match fs::read_to_string(file_path) {
            Ok(contents) => {
                conf_proxies = Self::parse_content(contents)?;
            }
            Err(e) => {
                return Err(format!("Error reading file: {}", e).into())
            }
        }
        Ok(conf_proxies)
    }

    /// 从字符串中解析代理
    /// 1. 先尝试使用 yaml 格式解析
    /// 2. 尝试解析 base64 格式
    /// 3. 尝试使用纯链接格式解析
    pub fn parse_content(content: String) -> Result<Vec<Proxy>, Box<dyn std::error::Error>> {
        let mut conf_proxies: Vec<Proxy> = Vec::new();
        match Self::parse_yaml_content(&content) {
            Ok(proxies) => {
                conf_proxies = proxies;
            }
            Err(_) => {
                match Self::parse_base64_content(&content) {
                    Ok(proxies) => {
                        conf_proxies = proxies;
                    }
                    Err(_) => {
                        match Self::parse_links_content(&content) {
                            Ok(proxies) => {
                                conf_proxies = proxies;
                            }
                            Err(_) => {
                            }
                        }
                    }
                }
            }
        }
        Ok(conf_proxies)
    }

    fn parse_yaml_content(content: &str) -> Result<Vec<Proxy>, Box<dyn std::error::Error>> {
        let mut conf_proxies: Vec<Proxy> = Vec::new();
        let yaml = serde_yaml::from_str::<serde_json::Value>(&content)?;
        let proxies = yaml.get("proxies").or_else(|| yaml.get("Proxies"));
        match proxies {
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
        let base64 = base64decode(content.trim())?;
        base64.split("\n").filter(|line| !line.is_empty()).for_each(|line| {
            match Proxy::from_link(line.trim().to_string()) {
                Ok(proxy) => {
                    conf_proxies.push(proxy)
                }
                Err(e) => {
                    println!("{}", e);
                }
            }
        });
        Ok(conf_proxies)
    }

    fn parse_links_content(content: &str) -> Result<Vec<Proxy>, Box<dyn std::error::Error>> {
        let mut conf_proxies: Vec<Proxy> = Vec::new();
        let links = content.split("\n")
            .filter(|line| !line.is_empty())
            .map(|link| link.trim()).collect::<Vec<&str>>();
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

        for proxy in proxies {
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
    }

    // 通过配置格式，获取 clash 配置文件内容
    pub fn get_clash_config_content(config_path: String, new_proxies: &Vec<Proxy>) -> io::Result<String> {
        let mut file = File::open(config_path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        let mut yaml: Value = serde_yaml::from_str(&contents).expect("Failed to parse YAML");

        // 插入 proxies
        if let Some(proxies) = yaml.get_mut("proxies").and_then(Value::as_sequence_mut) {
            for proxy in new_proxies {
                proxies.push(Value::Mapping(serde_yaml::from_str::<Mapping>(&*proxy.to_json()?).unwrap()));
            }
        } else {
            println!("Failed to find 'proxies' in the YAML file");
        }

        // 处理 proxy-groups 逻辑
        if let Some(groups) = yaml.get_mut("proxy-groups").and_then(Value::as_sequence_mut) {
            for group in groups.iter_mut() {
                if let Some(group_map) = group.as_mapping_mut() {
                    if let Some(Value::String(filter)) = group_map.get(&Value::String("filter".to_string())) {
                        let regex = Regex::new(filter).expect("Invalid regex");
                        if let Some(proxies) = group_map.get_mut(&Value::String("proxies".to_string())).and_then(Value::as_sequence_mut) {
                            for proxy in new_proxies {
                                if regex.is_match(proxy.get_name()) {
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

    pub fn save_proxies_into_clash_file(proxies: &Vec<Proxy>, config_path: String, save_path: String) {
        let content = SubManager::get_clash_config_content(config_path, proxies).unwrap();
        let mut file = File::create(&save_path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_get_clash_config_content() {
        let path = "conf/clash_release.yaml";
        let mut proxies = SubManager::parse_from_path("/Users/reajason/RustroverProjects/clash-butler/subs/0c1149d13476bbe3b62eecb7c9b895f4").unwrap();
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

    #[test]
    #[ignore]
    fn test_parse_conf() {
        let path = Path::new("/Users/reajason/RustroverProjects/clash-butler/subs/d417717ed83bdabad1d310906a47a3a2");
        let proxies = SubManager::parse_from_path(path).unwrap();
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
        let content = String::from("ss://cmM0LW1kNToydnpobzU=@120.241.144.101:2410#name\n\
        ss://cmM0LW1kNToydnpobzU=@120.241.144.101:2410#name1\n\
        ss://cmM0LW1kNToydnpobzU=@120.241.144.101:2410#name1\n\
        ss://cmM0LW1kNToydnpobzU=@120.241.144.101:2410#name\n\
        ss://cmM0LW1kNToydnpobzU=@120.241.144.101:2410#xixi");

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
}
