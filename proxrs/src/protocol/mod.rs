mod hysteria2;
mod ss;
mod ssr;
mod trojan;
mod vless;
mod vmess;

use crate::protocol::hysteria2::Hysteria2;
use crate::protocol::ss::SS;
use crate::protocol::ssr::Ssr;
use crate::protocol::trojan::Trojan;
use crate::protocol::vless::Vless;
use crate::protocol::vmess::Vmess;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{json, Value};
use std::any::Any;
use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::{Hash, Hasher};
use std::str::FromStr;
use std::{fmt, format};

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

pub trait ProxyAdapter: ProxyAdapterClone {
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
    pub proxy_type: ProxyType,
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
        self.adapter.get_name()
    }

    pub fn set_name(&mut self, name: &str) {
        self.adapter.set_name(name);
    }

    pub fn get_server(&self) -> &str {
        self.adapter.get_server()
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        match self.adapter.to_json() {
            Ok(json) => {
                let mut json_value: Value = serde_json::from_str(&json)?;
                if let Value::Object(ref mut map) = json_value {
                    map.insert("type".to_string(), json!(self.proxy_type));
                }
                serde_json::to_string(&json_value)
            }
            Err(e) => Err(e),
        }
    }

    pub fn from_link(link: String) -> Result<Proxy, UnsupportedLinkError> {
        if link.starts_with("ss://") {
            Ok(Proxy::new(ProxyType::SS, Box::new(SS::from_link(link)?)))
        } else if link.starts_with("ssr://") {
            Ok(Proxy::new(ProxyType::SSR, Box::new(Ssr::from_link(link)?)))
        } else if link.starts_with("vmess://") {
            Ok(Proxy::new(
                ProxyType::Vmess,
                Box::new(Vmess::from_link(link)?),
            ))
        } else if link.starts_with("trojan://") {
            Ok(Proxy::new(
                ProxyType::Trojan,
                Box::new(Trojan::from_link(link)?),
            ))
        } else if link.starts_with("hysteria2://") {
            Ok(Proxy::new(
                ProxyType::Hysteria2,
                Box::new(Hysteria2::from_link(link)?),
            ))
        } else if link.starts_with("vless://") {
            Ok(Proxy::new(
                ProxyType::Vless,
                Box::new(Vless::from_link(link)?),
            ))
        } else {
            Err(UnsupportedLinkError {
                message: format!("Unsupported link format: {}", link),
            })
        }
    }

    pub fn from_json(json: &str) -> Result<Proxy, UnsupportedLinkError> {
        let value = serde_json::from_str::<Value>(json).unwrap();
        if let Some(proxy_type) = value.get("type") {
            if proxy_type.as_str().unwrap() == "ss" {
                return match serde_json::from_str::<SS>(json) {
                    Ok(ss) => Ok(Proxy::new(ProxyType::SS, Box::new(ss))),
                    Err(e) => Err(UnsupportedLinkError {
                        message: format!("{}", e),
                    }),
                };
            } else if proxy_type.as_str().unwrap() == "ssr" {
                return match serde_json::from_str::<Ssr>(json) {
                    Ok(ssr) => Ok(Proxy::new(ProxyType::SSR, Box::new(ssr))),
                    Err(e) => Err(UnsupportedLinkError {
                        message: format!("{}", e),
                    }),
                };
            } else if proxy_type.as_str().unwrap() == "vmess" {
                return match serde_json::from_str::<Vmess>(json) {
                    Ok(vmess) => Ok(Proxy::new(ProxyType::Vmess, Box::new(vmess))),
                    Err(e) => Err(UnsupportedLinkError {
                        message: format!("{}", e),
                    }),
                };
            } else if proxy_type.as_str().unwrap() == "vless" {
                return match serde_json::from_str::<Vless>(json) {
                    Ok(vless) => Ok(Proxy::new(ProxyType::Vless, Box::new(vless))),
                    Err(e) => Err(UnsupportedLinkError {
                        message: format!("{}", e),
                    }),
                };
            } else if proxy_type.as_str().unwrap() == "trojan" {
                return match serde_json::from_str::<Trojan>(json) {
                    Ok(trojan) => Ok(Proxy::new(ProxyType::Trojan, Box::new(trojan))),
                    Err(e) => Err(UnsupportedLinkError {
                        message: format!("{}", e),
                    }),
                };
            } else if proxy_type.as_str().unwrap() == "hysteria2" {
                return match serde_json::from_str::<Hysteria2>(json) {
                    Ok(hysteria2) => Ok(Proxy::new(ProxyType::Hysteria2, Box::new(hysteria2))),
                    Err(e) => Err(UnsupportedLinkError {
                        message: format!("{}", e),
                    }),
                };
            }
        } else {
            return Err(UnsupportedLinkError {
                message: format!("proxy_type fetch error {}", json),
            });
        }
        Err(UnsupportedLinkError {
            message: json.to_string(),
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

impl Clone for Proxy {
    fn clone(&self) -> Self {
        Proxy {
            proxy_type: self.proxy_type.clone(), // 确保 ProxyType 实现了 Clone
            adapter: self.adapter.clone(),       // 使用 adapter 的 clone_box 方法
        }
    }
}

pub fn deserialize_u16_or_string<'de, D>(deserializer: D) -> Result<u16, D::Error>
where
    D: Deserializer<'de>,
{
    let value: Value = Deserialize::deserialize(deserializer)?;
    match value {
        Value::Number(num) => num
            .as_u64()
            .and_then(|n| u16::try_from(n).ok())
            .ok_or_else(|| serde::de::Error::custom("Invalid u16 value")),
        Value::String(s) => u16::from_str(&s).map_err(serde::de::Error::custom),
        _ => Err(serde::de::Error::custom("Expected a string or number")),
    }
}

#[cfg(test)]
mod test {
    use super::*;

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
        assert_eq!(
            Proxy::from_link(ssr_link).unwrap().proxy_type,
            ProxyType::SSR
        );

        let hysteria2_link = "hysteria2://bfbe4deb-07c8-450b-945e-e3c7676ba5ed@163.123.192.167:50000/?insecure=1&sni=www.microsoft.com&mport=50000-50080#%E5%89%A9%E4%BD%99%E6%B5%81%E9%87%8F%EF%BC%9A163.97%20GB".to_string();
        assert_eq!(
            Proxy::from_link(hysteria2_link).unwrap().proxy_type,
            ProxyType::Hysteria2
        );

        let trojan_link = "trojan://4fee57cc-ee15-4800-888f-3493f7b261f2@hk1.ee2c9087-71b0-70af-7924-09d714b25b96.6df03129.the-best-airport.com:443?type=tcp&sni=new.download.the-best-airport.com&allowInsecure=1#%F0%9F%87%AD%F0%9F%87%B0%E9%A6%99%E6%B8%AF%2001%20%7C%20%E4%B8%93%E7%BA%BF%0D".to_string();
        assert_eq!(
            Proxy::from_link(trojan_link).unwrap().proxy_type,
            ProxyType::Trojan
        );

        let vmess_link = "vmess://eyJ2IjoiMiIsInBzIjoiQHZwbnBvb2wiLCJhZGQiOiJrci5haWt1bmFwcC5jb20iLCJwb3J0IjoyMDAwNiwiaWQiOiIyMTM2ZGM2Yy01ZmQ0LTRiZmQtODhhMS0yYWVlYTk4ODhmOGIiLCJhaWQiOjAsInNjeSI6ImF1dG8iLCJuZXQiOiIiLCJ0bHMiOiIifQ==".to_string();
        assert_eq!(
            Proxy::from_link(vmess_link).unwrap().proxy_type,
            ProxyType::Vmess
        );

        let vless_link = "vless://2cd6ed0f-636e-4e6c-9449-5a263d7a0fa5@192.9.165.253:20001?encryption=none&security=tls&sni=cfed.tgzdyz2.top&fp=random&type=ws&host=cfed.tgzdyz2.top&path=%2FTG%40ZDYZ2%3Fed%3D2560#TG%40ZDYZ2%20-%E6%BE%B3%E5%A4%A7%E5%88%A9%E4%BA%9A%F0%9F%87%A6%F0%9F%87%BA".to_string();
        assert_eq!(
            Proxy::from_link(vless_link).unwrap().proxy_type,
            ProxyType::Vless
        );
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
}
