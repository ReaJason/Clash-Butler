mod ss;
mod ssr;
mod vmess;
mod trojan;
mod vless;
mod hysteria2;

use crate::proxy::hysteria2::Hysteria2;
use crate::proxy::ss::SS;
use crate::proxy::ssr::SSR;
use crate::proxy::trojan::Trojan;
use crate::proxy::vless::Vless;
use crate::proxy::vmess::Vmess;
use base64::prelude::BASE64_STANDARD;
use base64::{DecodeError, Engine};
use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{json, Value};
use std::any::Any;
use std::collections::{HashMap};
use std::fmt::Debug;
use std::path::Path;
use std::{fmt, format, fs};
use std::hash::{Hash, Hasher};
use std::str::FromStr;

#[derive(Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Ord, PartialOrd, Clone)]
pub enum ProxyType {
    #[serde(rename = "ss")]
    SS,
    #[serde(rename = "ssr")]
    SSR,
    #[serde(rename = "vmess")]
    Vmess,
    #[serde(rename = "vless")]
    Vless,
    #[serde(rename = "trojan")]
    Trojan,
    #[serde(rename = "hysteria2")]
    Hysteria2,
    #[serde(rename = "hysteria")]
    Hysteria,
    #[serde(rename = "wireguard")]
    WireGuard,
    #[serde(rename = "unknown")]
    Unknown,
}

#[derive(Deserialize, Debug, Serialize, Clone, PartialEq, Eq)]
pub struct WSOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    headers: Option<HashMap<String, String>>,
}

#[derive(Deserialize, Debug, Serialize, Clone, PartialEq, Eq)]
pub struct RealtyOptions {
    #[serde(skip_serializing_if = "Option::is_none", rename = "public-key")]
    pub public_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "short-id")]
    pub short_id: Option<String>,
}

#[derive(Deserialize, Debug, Serialize, Clone, PartialEq, Eq)]
pub struct GrpcOptions {
    #[serde(skip_serializing_if = "Option::is_none", rename = "grpc-service-name")]
    pub grpc_service_name: Option<String>,
}

fn add_base64_padding(content: &str) -> String {
    let mut padded = content.to_string();
    while padded.len() % 4 != 0 {
        padded.push('=');
    }
    padded
}

fn base64decode(content: &str) -> Result<String, DecodeError> {
    let padded_content = add_base64_padding(content);
    match BASE64_STANDARD.decode(padded_content.as_bytes()) {
        Ok(data) => {
            Ok(String::from_utf8(data).unwrap())
        }
        Err(e) => {
            Err(e)
        }
    }
}

fn base64encode(content: String) -> String {
    let b: &[u8] = content.as_bytes();
    BASE64_STANDARD.encode(b)
}

#[derive(Debug)]
pub struct UnsupportedLinkError {
    message: String,
}

impl fmt::Display for UnsupportedLinkError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for UnsupportedLinkError {}

pub trait ProxyAdapter : ProxyAdapterClone {
    fn get_name(&self) -> &str;
    fn set_name(&mut self, name: &str);
    fn get_server(&self) -> &str;
    fn to_link(&self) -> String;
    fn from_link(link: String) -> Result<Self, UnsupportedLinkError>
    where
        Self: Sized;

    fn to_json(&self) -> Result<String, serde_json::Error>;

    fn as_any(&self) -> &dyn Any;

    fn eq(&self, other: &dyn ProxyAdapter) -> bool;

    fn hash(&self, state: &mut dyn Hasher);
}


// 为 ProxyAdapter 增加 clone_box 方法
pub trait ProxyAdapterClone {
    fn clone_box(&self) -> Box<dyn ProxyAdapter>;
}

impl<T> ProxyAdapterClone for T
where
    T: 'static + ProxyAdapter + Clone,
{
    fn clone_box(&self) -> Box<dyn ProxyAdapter> {
        Box::new(self.clone())
    }
}

impl Clone for Box<dyn ProxyAdapter> {
    fn clone(&self) -> Box<dyn ProxyAdapter> {
        self.clone_box()
    }
}

pub struct Proxy {
    pub(crate) proxy_type: ProxyType,
    pub adapter: Box<dyn ProxyAdapter>,
}

impl Proxy {
    fn new(proxy_type: ProxyType, proxy_adapter: Box<dyn ProxyAdapter>) -> Proxy {
        Proxy {
            proxy_type,
            adapter: proxy_adapter,
        }
    }

    pub fn get_name(&self) -> &str {
        &self.adapter.get_name()
    }

    pub fn set_name(&mut self, name: &str) {
        self.adapter.set_name(name);
    }

    pub fn get_server(&self) -> &str {
        &self.adapter.get_server()
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        match self.adapter.to_json() {
            Ok(json) => {
                let mut json_value: Value = serde_json::from_str(&json)?;
                if let Value::Object(ref mut map) = json_value {
                    // TODO: type need to be first key
                    map.insert("type".to_string(), json!(self.proxy_type));
                } else {
                    return Err(serde_json::Error::custom("JSON is not an object"));
                }
                serde_json::to_string(&json_value)
            }
            Err(e) => { Err(e) }
        }
    }

    pub fn from_link(link: String) -> Result<Proxy, UnsupportedLinkError> {
        if link.starts_with("ss://") {
            Ok(Proxy::new(ProxyType::SS, Box::new(SS::from_link(link)?)))
        } else if link.starts_with("ssr://") {
            Ok(Proxy::new(ProxyType::SSR, Box::new(SSR::from_link(link)?)))
        } else if link.starts_with("vmess://") {
            Ok(Proxy::new(ProxyType::Vmess, Box::new(Vmess::from_link(link)?)))
        } else if link.starts_with("trojan://") {
            Ok(Proxy::new(ProxyType::Trojan, Box::new(Trojan::from_link(link)?)))
        } else if link.starts_with("hysteria2://") {
            Ok(Proxy::new(ProxyType::Hysteria2, Box::new(Hysteria2::from_link(link)?)))
        } else if link.starts_with("vless://") {
            Ok(Proxy::new(ProxyType::Vless, Box::new(Vless::from_link(link)?)))
        } else {
            Err(UnsupportedLinkError {
                message: format!("Unsupported link format: {}", link),
            })
        }
    }

    pub fn from_json(json: Value) -> Result<Proxy, UnsupportedLinkError> {
        if let Some(proxy_type) = json.get("type") {
            if proxy_type.as_str().unwrap() == "ss" {
                return match serde_json::from_value::<SS>(json) {
                    Ok(ss) => {
                        Ok(Proxy::new(ProxyType::SS, Box::new(ss)))
                    }
                    Err(e) => {
                        Err(UnsupportedLinkError {
                            message: format!("{}", e),
                        })
                    }
                };
            } else if proxy_type.as_str().unwrap() == "ssr" {
                return match serde_json::from_value::<SSR>(json) {
                    Ok(ssr) => {
                        Ok(Proxy::new(ProxyType::SSR, Box::new(ssr)))
                    }
                    Err(e) => {
                        Err(UnsupportedLinkError {
                            message: format!("{}", e),
                        })
                    }
                };
            } else if proxy_type.as_str().unwrap() == "vmess" {
                return match serde_json::from_value::<Vmess>(json) {
                    Ok(vmess) => {
                        Ok(Proxy::new(ProxyType::Vmess, Box::new(vmess)))
                    }
                    Err(e) => {
                        Err(UnsupportedLinkError {
                            message: format!("{}", e),
                        })
                    }
                };
            } else if proxy_type.as_str().unwrap() == "vless" {
                return match serde_json::from_value::<Vless>(json) {
                    Ok(vless) => {
                        Ok(Proxy::new(ProxyType::Vless, Box::new(vless)))
                    }
                    Err(e) => {
                        Err(UnsupportedLinkError {
                            message: format!("{}", e),
                        })
                    }
                };
            } else if proxy_type.as_str().unwrap() == "trojan" {
                return match serde_json::from_value::<Trojan>(json) {
                    Ok(trojan) => {
                        Ok(Proxy::new(ProxyType::Trojan, Box::new(trojan)))
                    }
                    Err(e) => {
                        Err(UnsupportedLinkError {
                            message: format!("{}", e),
                        })
                    }
                };
            } else if proxy_type.as_str().unwrap() == "hysteria2" {
                return match serde_json::from_value::<Hysteria2>(json) {
                    Ok(hysteria2) => {
                        Ok(Proxy::new(ProxyType::Hysteria2, Box::new(hysteria2)))
                    }
                    Err(e) => {
                        Err(UnsupportedLinkError {
                            message: format!("{}", e),
                        })
                    }
                };
            }
        } else {
            return Err(UnsupportedLinkError {
                message: format!("proxy_type fetch error {}", json),
            });
        }
        Err(UnsupportedLinkError {
            message: "".to_string(),
        })
    }
}

impl PartialEq for Proxy {
    fn eq(&self, other: &Self) -> bool {
        self.proxy_type == other.proxy_type && self.adapter.eq(other.adapter.as_ref())
    }
}

impl Eq for Proxy {}

impl Hash for Proxy {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.proxy_type.hash(state);
        self.adapter.hash(state);
    }
}

impl Debug for Proxy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.to_json().unwrap().as_str())
    }
}

// 为 Proxy 结构体实现 Clone
impl Clone for Proxy {
    fn clone(&self) -> Self {
        Proxy {
            proxy_type: self.proxy_type.clone(), // 确保 ProxyType 实现了 Clone
            adapter: self.adapter.clone(), // 使用 adapter 的 clone_box 方法
        }
    }
}


pub fn parse_conf<P: AsRef<Path>>(file_path: P) -> Result<Vec<Proxy>, Box<dyn std::error::Error>> {
    let mut conf_proxies: Vec<Proxy> = Vec::new();
    match fs::read_to_string(file_path) {
        Ok(contents) => {
            match parse_yaml_content(&contents) {
                Ok(proxies) => {
                    conf_proxies = proxies;
                }
                Err(_) => {
                    println!("try parse yaml file failed");
                    match parse_base64_content(&contents) {
                        Ok(proxies) => {
                            conf_proxies = proxies;
                        }
                        Err(e) => {
                            println!("{}", e);
                            println!("try parse base64 file failed");
                        }
                    }
                }
            }
        }
        Err(e) => {
            return Err(format!("Error reading file: {}", e).into())
        }
    }
    Ok(conf_proxies)
}

pub fn parse_yaml_content(content: &str) -> Result<Vec<Proxy>, Box<dyn std::error::Error>> {
    let mut conf_proxies: Vec<Proxy> = Vec::new();
    let yaml = serde_yaml::from_str::<Value>(&content)?;
    let proxies = yaml.get("proxies").or_else(|| yaml.get("Proxies"));
    match proxies {
        None => {
            return Err(format!("Proxy not found: {}", content).into());
        }
        Some(proxies) => {
            if let Some(proxies_arr) = proxies.as_array() {
                for proxy in proxies_arr {
                    let result = Proxy::from_json(proxy.clone());
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

pub fn parse_base64_content(content: &str) -> Result<Vec<Proxy>, Box<dyn std::error::Error>> {
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


pub fn deserialize_u16_or_string<'de, D>(deserializer: D) -> Result<u16, D::Error>
where
    D: Deserializer<'de>,
{
    let value: serde_json::Value = Deserialize::deserialize(deserializer)?;
    match value {
        serde_json::Value::Number(num) => {
            // If the value is already a number, try converting to u16
            num.as_u64()
                .and_then(|n| u16::try_from(n).ok())
                .ok_or_else(|| serde::de::Error::custom("Invalid u16 value"))
        }
        serde_json::Value::String(s) => {
            // If the value is a string, try parsing it as u16
            u16::from_str(&s).map_err(serde::de::Error::custom)
        }
        _ => Err(serde::de::Error::custom("Expected a string or number")),
    }
}

#[cfg(test)]
mod test {
    use regex::Regex;
    use super::*;

    #[test]
    fn test_base64() {
        // println!("{}", base64decode(String::from("c3M6Ly9ZV1Z6TFRFeU9DMW5ZMjA2WkRsak5UYzNNekk0Wm1Jek5EbG1aUT09QDEyMC4yMzIuNzMuNjg6NDA2NzYjJUYwJTlGJTg3JUFEJUYwJTlGJTg3JUIwSEsNCnNzOi8vWVdWekxURXlPQzFuWTIwNlpEbGpOVGMzTXpJNFptSXpORGxtWlE9PUAxMjAuMjMyLjczLjY4OjQ3MDM0IyVGMCU5RiU4NyVBRCVGMCU5RiU4NyVCMEhLDQpzczovL1lXVnpMVEV5T0MxblkyMDZaRGxqTlRjM016STRabUl6TkRsbVpRPT1AMTIwLjIzMi43My42ODo0MzI5NiMlRjAlOUYlODclQUQlRjAlOUYlODclQjBISw0Kc3M6Ly9ZV1Z6TFRFeU9DMW5ZMjA2WkRsak5UYzNNekk0Wm1Jek5EbG1aUT09QDEyMC4yMzIuNzMuNjg6NDQ0MjEjJUYwJTlGJTg3JUFEJUYwJTlGJTg3JUIwSEsNCnNzOi8vWVdWekxURXlPQzFuWTIwNlpEbGpOVGMzTXpJNFptSXpORGxtWlE9PUAxMjAuMjMyLjczLjY4OjQzMDg4IyVGMCU5RiU4NyVBRCVGMCU5RiU4NyVCMEhLDQpzczovL1lXVnpMVEV5T0MxblkyMDZaRGxqTlRjM016STRabUl6TkRsbVpRPT1AMTIwLjIzMi43My42ODo0NzE1MCMlRjAlOUYlODclQUQlRjAlOUYlODclQjBISw0Kc3M6Ly9ZV1Z6TFRFeU9DMW5ZMjA2WkRsak5UYzNNekk0Wm1Jek5EbG1aUT09QDEyMC4yMzIuNzMuNjg6NDM2NjQjJUYwJTlGJTg3JUFEJUYwJTlGJTg3JUIwSEsNCnNzOi8vWVdWekxURXlPQzFuWTIwNlpEbGpOVGMzTXpJNFptSXpORGxtWlE9PUAxMjAuMjMyLjczLjY4OjQwMTA3IyVGMCU5RiU4NyVBRCVGMCU5RiU4NyVCMEhLDQpzczovL1lXVnpMVEV5T0MxblkyMDZaRGxqTlRjM016STRabUl6TkRsbVpRPT1AMTIwLjIzMi43My42ODo0MTgzMiMlRjAlOUYlODclQUQlRjAlOUYlODclQjBISw0Kc3M6Ly9ZV1Z6TFRFeU9DMW5ZMjA2WkRsak5UYzNNekk0Wm1Jek5EbG1aUT09QDEyMC4yMzIuNzMuNjg6NDM2NDEjJUYwJTlGJTg3JUFEJUYwJTlGJTg3JUIwSEsNCnNzOi8vWVdWekxURXlPQzFuWTIwNlpEbGpOVGMzTXpJNFptSXpORGxtWlE9PUAyMTEuOTkuMTAyLjIyNDo0MDY3NiMlRjAlOUYlODclQUQlRjAlOUYlODclQjBISw0Kc3M6Ly9ZV1Z6TFRFeU9DMW5ZMjA2WkRsak5UYzNNekk0Wm1Jek5EbG1aUT09QDIxMS45OS4xMDIuMjI0OjQ3MDM0IyVGMCU5RiU4NyVBRCVGMCU5RiU4NyVCMEhLDQpzczovL1lXVnpMVEV5T0MxblkyMDZaRGxqTlRjM016STRabUl6TkRsbVpRPT1AMjExLjk5LjEwMi4yMjQ6NDMyOTYjJUYwJTlGJTg3JUFEJUYwJTlGJTg3JUIwSEsNCnNzOi8vWVdWekxURXlPQzFuWTIwNlpEbGpOVGMzTXpJNFptSXpORGxtWlE9PUAyMTEuOTkuMTAyLjIyNDo0NDQyMSMlRjAlOUYlODclQUQlRjAlOUYlODclQjBISw0Kc3M6Ly9ZV1Z6TFRFeU9DMW5ZMjA2WkRsak5UYzNNekk0Wm1Jek5EbG1aUT09QDIxMS45OS4xMDIuMjI0OjQzMDg4IyVGMCU5RiU4NyVBRCVGMCU5RiU4NyVCMEhLDQpzczovL1lXVnpMVEV5T0MxblkyMDZaRGxqTlRjM016STRabUl6TkRsbVpRPT1AMjExLjk5LjEwMi4yMjQ6NDcxNTAjJUYwJTlGJTg3JUFEJUYwJTlGJTg3JUIwSEsNCnNzOi8vWVdWekxURXlPQzFuWTIwNlpEbGpOVGMzTXpJNFptSXpORGxtWlE9PUAyMTEuOTkuMTAyLjIyNDo0MzY2NCMlRjAlOUYlODclQUQlRjAlOUYlODclQjBISw0Kc3M6Ly9ZV1Z6TFRFeU9DMW5ZMjA2WkRsak5UYzNNekk0Wm1Jek5EbG1aUT09QDIxMS45OS4xMDIuMjI0OjQwMTA3IyVGMCU5RiU4NyVBRCVGMCU5RiU4NyVCMEhLDQpzczovL1lXVnpMVEV5T0MxblkyMDZaRGxqTlRjM016STRabUl6TkRsbVpRPT1AMjExLjk5LjEwMi4yMjQ6NDE4MzIjJUYwJTlGJTg3JUFEJUYwJTlGJTg3JUIwSEsNCnNzOi8vWVdWekxURXlPQzFuWTIwNlpEbGpOVGMzTXpJNFptSXpORGxtWlE9PUAyMTEuOTkuMTAyLjIyNDo0MzY0MSMlRjAlOUYlODclQUQlRjAlOUYlODclQjBISw0Kc3M6Ly9ZV1Z6TFRFeU9DMW5ZMjA2WkRsak5UYzNNekk0Wm1Jek5EbG1aUT09QDEyMC4yMDQuOTQuMzA6NDYxMDEjJUYwJTlGJTg3JUJBJUYwJTlGJTg3JUI4VVMNCnNzOi8vWVdWekxURXlPQzFuWTIwNlpEbGpOVGMzTXpJNFptSXpORGxtWlE9PUAxMjAuMjA0Ljk0LjMwOjQzNzkxIyVGMCU5RiU4NyVCQSVGMCU5RiU4NyVCOFVTDQpzczovL1lXVnpMVEV5T0MxblkyMDZaRGxqTlRjM016STRabUl6TkRsbVpRPT1AMTIwLjIwNC45NC4zMDo0MjI4NSMlRjAlOUYlODclQkElRjAlOUYlODclQjhVUw0Kc3M6Ly9ZV1Z6TFRFeU9DMW5ZMjA2WkRsak5UYzNNekk0Wm1Jek5EbG1aUT09QDEyMC4yMDQuOTQuMzA6NDY4MjQjJUYwJTlGJTg3JUJBJUYwJTlGJTg3JUI4VVMNCg").as_str()).unwrap());
        println!("{}", base64decode("eyJ2IjoiMiIsInBzIjoiXHU1MjY5XHU0ZjU5XHU2ZDQxXHU5MWNmXHVmZjFhNzkuNTEgR0IiLCJhZGQiOiJjZG5jZG5jZG5jZG4uNzg0NjU0Lnh5eiIsInBvcnQiOiIyMDUyIiwiaWQiOiIzZWE1NzhjNi0xZWFhLTRlMTUtYmZlMS05Zjc1N2I1OGU4ZjIiLCJhaWQiOiIwIiwibmV0Ijoid3MiLCJ0eXBlIjoibm9uZSIsImhvc3QiOiJjYS1jZmNkbi5haWt1bmFwcC5jb20iLCJwYXRoIjoiXC9pbmRleD9lZD0yMDQ4IiwidGxzIjoiIn0=").unwrap());
    }

    #[test]
    fn test_base64decode() {
        match base64decode(String::from("aGVsbG8=").as_str()) {
            Ok(str) => {
                assert_eq!(str, String::from("hello"))
            }
            Err(_) => {
                assert!(false)
            }
        }
    }

    #[test]
    fn test_base64decode_error() {
        let Err(e) = base64decode(String::from("aGVsbG").as_str()) else { todo!() };
        assert!(matches!(e, DecodeError::InvalidPadding))
    }

    #[test]
    fn test_parse_json() {
        let json = "{\"name\":\"123123\",\"server\":\"aliyun.2096.us.kg\",\"port\":2096,\"client-fingerprint\":\"random\",\"type\":\"vless\",\"uuid\":\"99280094-e683-476b-a3cd-0d37c3892c6f\",\"tls\":true,\"tfo\":false,\"skip-cert-verify\":true,\"servername\":\"syvless.6516789.xyz\",\"network\":\"ws\",\"ws-opts\":{\"path\":\"/?proxyip\\u003doracle.gitgoogle.com\",\"headers\":{\"Host\":\"syvless.6516789.xyz\"}},\"udp\":true}";
        let value = serde_json::from_str::<Value>(json).unwrap();
        println!("{}", value.get("name").unwrap());
        println!("{}", value.get("type").unwrap());
        println!("{:?}", serde_json::from_value::<Vless>(value).unwrap());
    }

    #[test]
    fn test_proxy_type() {
        let ss_link = "ss://YWVzLTEyOC1nY206ZDljNTc3MzI4ZmIzNDlmZQ==@120.232.73.68:40676#%F0%9F%87%AD%F0%9F%87%B0HK".to_string();
        assert_eq!(Proxy::from_link(ss_link).unwrap().proxy_type, ProxyType::SS);

        let ssr_link = "ssr://dmlwLmJhc2ljbm9kZS5ob3N0OjExODQ1OmF1dGhfYWVzMTI4X3NoYTE6Y2hhY2hhMjAtaWV0Zjp0bHMxLjJfdGlja2V0X2F1dGg6Um1oaVpUQjYvP3JlbWFya3M9VUhKdkxlbW1tZWE0cnlCSVMwZmt1S2psaGFqb3A2UHBsSUhrdUtoQk1nPT0mb2Jmc3BhcmFtPU5tWTBNV0l5TkM1dGFXTnliM052Wm5RdVkyOXQmcHJvdG9wYXJhbT1NalE2VTNCWlZYUlFaVXBaYUZKck5FWlhRdz09".to_string();
        assert_eq!(Proxy::from_link(ssr_link).unwrap().proxy_type, ProxyType::SSR);

        let hysteria2_link = "hysteria2://bfbe4deb-07c8-450b-945e-e3c7676ba5ed@163.123.192.167:50000/?insecure=1&sni=www.microsoft.com&mport=50000-50080#%E5%89%A9%E4%BD%99%E6%B5%81%E9%87%8F%EF%BC%9A163.97%20GB".to_string();
        assert_eq!(Proxy::from_link(hysteria2_link).unwrap().proxy_type, ProxyType::Hysteria2);

        let trojan_link = "trojan://4fee57cc-ee15-4800-888f-3493f7b261f2@hk1.ee2c9087-71b0-70af-7924-09d714b25b96.6df03129.the-best-airport.com:443?type=tcp&sni=new.download.the-best-airport.com&allowInsecure=1#%F0%9F%87%AD%F0%9F%87%B0%E9%A6%99%E6%B8%AF%2001%20%7C%20%E4%B8%93%E7%BA%BF%0D".to_string();
        assert_eq!(Proxy::from_link(trojan_link).unwrap().proxy_type, ProxyType::Trojan);

        let vmess_link = "vmess://eyJ2IjoiMiIsInBzIjoiQHZwbnBvb2wiLCJhZGQiOiJrci5haWt1bmFwcC5jb20iLCJwb3J0IjoyMDAwNiwiaWQiOiIyMTM2ZGM2Yy01ZmQ0LTRiZmQtODhhMS0yYWVlYTk4ODhmOGIiLCJhaWQiOjAsInNjeSI6ImF1dG8iLCJuZXQiOiIiLCJ0bHMiOiIifQ==".to_string();
        assert_eq!(Proxy::from_link(vmess_link).unwrap().proxy_type, ProxyType::Vmess);

        let vless_link = "vless://2cd6ed0f-636e-4e6c-9449-5a263d7a0fa5@192.9.165.253:20001?encryption=none&security=tls&sni=cfed.tgzdyz2.top&fp=random&type=ws&host=cfed.tgzdyz2.top&path=%2FTG%40ZDYZ2%3Fed%3D2560#TG%40ZDYZ2%20-%E6%BE%B3%E5%A4%A7%E5%88%A9%E4%BA%9A%F0%9F%87%A6%F0%9F%87%BA".to_string();
        assert_eq!(Proxy::from_link(vless_link).unwrap().proxy_type, ProxyType::Vless);
    }

    #[test]
    fn test_proxy() {
        let link = "ss://YWVzLTEyOC1nY206ZDljNTc3MzI4ZmIzNDlmZQ==@120.232.73.68:40676#%F0%9F%87%AD%F0%9F%87%B0HK".to_string();
        let proxy1 = Proxy::from_link(link.clone()).unwrap();
        let proxy2 = Proxy::from_link(link.clone()).unwrap();
        println!("{:?}", proxy1);
        println!("{:?}", proxy2);
        assert_eq!(proxy1, proxy2);
    }

    #[test]
    fn test_parser_conf() {
        let parent = Path::new("/Users/reajason/RustroverProjects/clash-butler/subs");
        for entry in fs::read_dir(parent).unwrap() {
            if let Ok(entry) = entry {
                let path = entry.path();
                if path.is_file() {
                    let proxies = parse_conf(&path).unwrap();
                    println!("{:?}", path);
                    assert_ne!(proxies.len(), 0);
                }
            }
        }
    }

    #[test]
    fn test_parse_conf() {
        let path = Path::new("/Users/reajason/RustroverProjects/clash-butler/subs/d417717ed83bdabad1d310906a47a3a2");
        let proxies = parse_conf(path).unwrap();
        for proxy in &proxies {
            println!("{:?}", proxy);
        }
    }

    #[test]
    fn main() {
        // 首先，我们需要在 Cargo.toml 中添加 regex 依赖
        // [dependencies]
        // regex = "1.5.4"

        // 创建正则表达式
        let re = Regex::new(r"(?i)港|hk|hongkong|hong kong").unwrap();

        // 测试一些字符串
        let test_strings = vec![
            "香港",
            "HK",
            "hongkong",
            "Hong Kong",
            "HONG KONG",
            "Tokyo",
            "hk island",
        ];

        for s in test_strings {
            if re.is_match(s) {
                println!("'{}' matches the pattern", s);
            } else {
                println!("'{}' does not match the pattern", s);
            }
        }

        // 如果我们想提取匹配的部分
        let text = "I love Hong Kong and HK is great!";
        for cap in re.find_iter(text) {
            println!("Found match: {} at position {:?}", cap.as_str(), cap.range());
        }
    }
}