use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::io::{Read, Write};
use std::time::Duration;
use std::{env, io};

use crate::proxy::{parse_conf, Proxy};
use regex::Regex;
use reqwest::Client;
use serde_yaml::{Mapping, Value};
use tokio::time::sleep;
use tracing::{error, info};

#[derive(Debug)]
pub struct SubConverter {}

impl SubConverter {
    pub async fn get_proxies(subs: &Vec<String>) -> Vec<Proxy> {
        let mut proxies: Vec<Proxy> = Vec::new();
        for url in subs {
            if url.starts_with("http") {
                match download_new_sub(&url).await {
                    Ok(file_path) => {
                        proxies.extend(parse_conf(file_path).unwrap());
                    }
                    Err(e) => {
                        error!(e)
                    }
                }
            } else {
                proxies.extend(parse_conf(url).unwrap());
            }
        }

        if !proxies.is_empty() {
            proxies = Self::exclude_dup_proxies(proxies);
            Self::rename_dup_proxies_name(&mut proxies);
        }

        proxies
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
        for proxy in proxies {
            let mut name = proxy.get_name().to_string();
            let count = name_counts.entry(name.clone()).or_insert(0);
            if *count > 0 {
                name = format!("{}{}", name, count);
            }
            proxy.set_name(&name);
            *count += 1;
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
}

pub async fn download_new_sub(sub_url: &str) -> Result<String, Box<dyn std::error::Error>> {
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
                    let re = Regex::new(r"/files/(.*?)/raw").unwrap();
                    let uuid = re.captures(sub_url)
                        .and_then(|caps| caps.get(1))
                        .map_or_else(|| {
                            format!("{:x}", md5::compute(sub_url))
                        }, |m| m.as_str().to_string());

                    let file_path = format!("subs/{}", uuid);
                    info!("sub download success in {}", file_path);
                    let mut file = File::create(&file_path).unwrap();

                    let content_result = resp.text().await;
                    match content_result {
                        Ok(content) => {
                            file.write_all(content.as_bytes()).unwrap();
                            Ok(env::current_dir().unwrap().join(file_path).to_string_lossy().to_string())
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


pub fn include_names(proxies: Vec<Proxy>, names: Vec<String>) -> Vec<Proxy> {
    let mut release_proxies = Vec::new();

    for proxy in proxies {
        if names.contains(&proxy.get_name().to_string()) {
            release_proxies.push(proxy);
        }
    }
    release_proxies
}

pub fn save_proxies_into_clash_file(proxies: &Vec<Proxy>, config_path: String, save_path: String) {
    let content = SubConverter::get_clash_config_content(config_path, proxies).unwrap();
    let mut file = File::create(&save_path).unwrap();
    file.write_all(content.as_bytes()).unwrap();
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::proxy::parse_conf;

    #[test]
    fn test_get_clash_config_content() {
        let path = "conf/clash_release.yaml";
        let mut proxies = parse_conf("/Users/reajason/RustroverProjects/clash-butler/subs/0c1149d13476bbe3b62eecb7c9b895f4").unwrap();
        SubConverter::unset_proxies_name(&mut proxies);
        let content = SubConverter::get_clash_config_content(path.to_string(), &proxies).expect("TODO: panic message");
        println!("{}", content);
    }
}