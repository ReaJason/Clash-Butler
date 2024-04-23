use std::{env, fs};
use std::fs::File;
use std::io::Write;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use regex::Regex;
use reqwest::Client;
use tokio::time::sleep;
use tracing::{error, info};

#[derive(Debug)]
pub struct SubConverter {
    pub port: u64,
    url: String,
    process: Option<Child>,
    core_path: String,
    log_path: String,
    config_path: String,
}

impl SubConverter {
    pub fn new(port: u64) -> Self {
        let converter = SubConverter {
            port,
            url: format!("http://127.0.0.1:{}", port),
            process: None,
            core_path: "subconverter/subconverter".to_string(),
            log_path: "logs/sub.log".to_string(),
            config_path: "subconverter/pref.toml".to_string(),
        };
        converter.change_sub_converter_server_port()
            .expect("修改 subconverter 端口失败");
        converter
    }

    pub async fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let log_file = File::create(&self.log_path)?;

        Command::new("echo")
            .arg("Hello, world!")
            .stdout(Stdio::from(File::create("logs/test.log")?))
            .spawn()?;


        let sub_converter_process = Command::new(&self.core_path)
            .stdout(Stdio::from(log_file.try_clone()?))
            .stderr(Stdio::from(log_file))
            .spawn()?;

        tokio::time::sleep(Duration::from_secs(2)).await;

        let version_url = format!("{}/version", self.url);
        let response = reqwest::get(&version_url).await.unwrap();
        let version = response.text().await;
        info!("{} 服务已启动, {}", version.unwrap().trim(), version_url);

        self.process = Some(sub_converter_process);
        Ok(())
    }

    fn change_sub_converter_server_port(&self) -> Result<(), std::io::Error> {
        let mut perf = fs::read_to_string(&self.config_path)?;
        let port_regex = Regex::new(r"port\s=\s\d+").unwrap();
        perf = port_regex.replace(&perf, format!("port = {}", &self.port)).to_string();
        fs::write(&self.config_path, perf)?;
        Ok(())
    }

    pub fn stop(mut self) -> std::io::Result<()> {
        if let Some(mut process) = self.process.take() {
            process.kill()?;
            process.wait()?;
        }
        Ok(())
    }

    pub async fn get_clash_sub_url(&self, sub_config: SubConfig) -> String {
        let mut subs = vec![];
        for url in sub_config.urls {
            if url.starts_with("http") {
                match download_new_sub(&url).await {
                    Ok(file_path) => {
                        subs.push(file_path)
                    }
                    Err(e) => {
                        error!(e)
                    }
                }
            } else {
                subs.push(url);
            }
        }

        if subs.is_empty() {
            return "".to_string();
        }

        let mut url = format!("http://127.0.0.1:{}/clash?url={}&config={}",
                              &self.port,
                              urlencoding::encode(&subs.join("|")),
                              sub_config.config
        );

        if let Some(includes) = sub_config.includes {
            let include_str = includes.join("|");
            url = format!("{}&include={}", url, include_str);
        }

        if let Some(add_emoji) = sub_config.add_emoji {
            url = format!("{}&add_emoji={}", url, add_emoji);
        }

        if let Some(rename) = sub_config.rename {
            url = format!("{}&rename={}", url, rename.join("`"))
        }

        if let Some(mixed_port) = sub_config.mixed_port {
            url = format!("{}&clash.mixed={}", url, mixed_port)
        }

        if let Some(external_url) = sub_config.external_url {
            url = format!("{}&clash.external={}", url, external_url)
        }

        url
    }
}

async fn download_new_sub(sub_url: &str) -> Result<String, Box<dyn std::error::Error>> {
    let client = Client::new();
    let mut attempts = 0;
    let retries = 3;

    loop {
        let response = client
            .get(sub_url)
            .timeout(Duration::from_secs(10))
            .send()
            .await;

        match response {
            Ok(ref resp) => {
                return if resp.status().is_success() {
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

                    let content = response?.text().await.unwrap();
                    file.write_all(content.as_bytes()).unwrap();
                    Ok(env::current_dir().unwrap().join(file_path).to_string_lossy().to_string())
                } else {
                    return Err(format!("获取订阅连失败 {} 响应码 {}", sub_url, response?.status().as_str()).into());
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

#[derive(Debug, Default)]
pub struct SubConfig {
    pub urls: Vec<String>,
    pub config: String,
    pub includes: Option<Vec<String>>,
    pub add_emoji: Option<bool>,
    pub rename: Option<Vec<String>>,
    pub mixed_port: Option<u64>,
    pub external_url: Option<String>,
}


#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_start_sub_converter() {
        let mut subconverter = SubConverter::new(25500);
        subconverter.start().await.unwrap();
        subconverter.stop().unwrap();
        match reqwest::get("http://127.0.0.1:25500/version").await {
            Ok(_) => println!("subconverter 服务未正确关闭"),
            Err(_) => println!("subconverter 服务已关闭"),
        }
    }
}

