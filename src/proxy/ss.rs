use crate::base64::{base64decode, base64encode};
use crate::proxy::deserialize_u16_or_string;
use crate::proxy::{ProxyAdapter, UnsupportedLinkError};
use serde::{Deserialize, Serialize};
use serde_json::Error;
use std::any::Any;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

#[derive(Deserialize, Debug, Serialize, Eq, Clone)]
pub struct SS {
    pub name: String,
    pub server: String,
    #[serde(deserialize_with = "deserialize_u16_or_string")]
    pub port: u16,
    pub password: String,
    pub cipher: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plugin: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "plugin-opts")]
    pub plugin_opts: Option<HashMap<String, String>>,
}

impl PartialEq for SS {
    fn eq(&self, other: &Self) -> bool {
        self.server == self.server
            && self.port == other.port
            && self.password == other.password
    }
}

impl ProxyAdapter for SS {
    fn get_name(&self) -> &str {
        &self.name
    }

    fn set_name(&mut self, name: &str) {
        self.name = name.to_string();
    }

    fn get_server(&self) -> &str {
        &self.server
    }

    /// 将节点信息转为单个分享链接
    /// https://github.com/v2rayA/v2rayA/blob/main/service/core/serverObj/shadowsocks.go#L354
    fn to_link(&self) -> String {
        let cipher_pwd = base64encode(format!("{}:{}", &self.cipher, &self.password));
        let server_port = format!("{}:{}", &self.server, &self.port);
        if let Some(plugin) = &self.plugin {
            let mut plugin = format!("plugin={plugin};");
            if let Some(plugin_opts) = &self.plugin_opts {
                let str = plugin_opts
                    .iter()
                    .map(|(key, value)| {
                        let v = key.clone() + "=" + value;
                        urlencoding::encode(&v).into_owned()
                    })
                    .collect::<Vec<_>>().join(";");
                plugin.push_str(&str);
            }
            format!("ss://{}@{}?{}#{}", cipher_pwd, server_port, plugin, urlencoding::encode(&self.name))
        } else {
            format!("ss://{}@{}#{}", cipher_pwd, server_port, urlencoding::encode(&self.name))
        }
    }

    fn from_link(link: String) -> Result<Self, UnsupportedLinkError> {
        // ss://YWVzLTEyOC1nY206ZDljNTc3MzI4ZmIzNDlmZQ==@120.232.73.68:40676#%F0%9F%87%AD%F0%9F%87%B0HK
        let url: &str = &link[5..];

        // parse name
        // [YWVzLTEyOC1nY206ZDljNTc3MzI4ZmIzNDlmZQ==@120.232.73.68:40676, %F0%9F%87%AD%F0%9F%87%B0HK]
        let mut name = String::from("");
        let parts: Vec<&str> = url.split("#").collect();
        if parts.len() > 1 {
            name = urlencoding::decode(parts[1]).unwrap_or_default().trim().to_string();
        }

        // parse plugin
        // cmM0LW1kNToydnpobzU=@120.241.144.101:2410?plugin=obfs-local;obfs%3Dhttp;obfs-host%3D89c19109670.microsoft.com&group=QHZwbmhhdA
        let url = parts[0];
        let parts: Vec<&str> = url.split("?").collect();
        let mut plugin = None;
        let mut plugin_opts = None;
        if parts.len() > 1 {
            let params = parts[1];
            let mut params_map: HashMap<&str, String> = HashMap::new();
            for param in params.split("&") {
                if let Some((key, value)) = param.split_once('=') {
                    let value = value.parse::<String>().unwrap();
                    params_map.insert(key, value);
                }
            }

            if let Some(item) = params_map.get("plugin") {
                let plugin_params = item.split(";").collect::<Vec<_>>();
                plugin = Some(plugin_params[0].to_string());
                if plugin_params.len() > 1 {
                    let mut map: HashMap<String, String> = HashMap::new();
                    plugin_params[1..].iter().for_each(|param| {
                        let value = urlencoding::decode(param).unwrap_or_default().trim().to_string();
                        let kvs = value.split("=").collect::<Vec<_>>();
                        if kvs.len() == 2 {
                            map.insert(kvs[0].to_string(), kvs[1].to_string());
                        }
                    });
                    plugin_opts = Some(map);
                }
            }
        }

        // parse server port
        // YWVzLTEyOC1nY206ZDljNTc3MzI4ZmIzNDlmZQ==@120.232.73.68:40676
        let url = parts[0];
        let parts: Vec<&str> = url.split("@").collect();

        let mut cipher = String::from("");
        let mut password = String::from("");

        if let Ok(secret) = base64decode(parts[0]) {
            let parts: Vec<&str> = secret.split(":").collect();
            cipher = parts[0].parse().unwrap();
            password = parts[1].parse().unwrap();
        }

        let server_port = parts[1];
        let parts: Vec<&str> = server_port.split(":").collect();
        let server = parts[0].parse::<String>().unwrap();
        let port = parts[1].parse::<u16>().unwrap();

        Ok(SS {
            name,
            server,
            port,
            password,
            cipher,
            plugin,
            plugin_opts,
        })
    }

    fn to_json(&self) -> Result<String, Error> {
        serde_json::to_string(self)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn eq(&self, other: &dyn ProxyAdapter) -> bool {
        if let Some(other) = other.as_any().downcast_ref::<SS>() {
            self == other
        } else {
            false
        }
    }

    fn hash(&self, mut state: &mut dyn Hasher) {
        self.server.hash(&mut state);
        self.port.hash(&mut state);
        self.password.hash(&mut state);
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::proxy::ProxyAdapter;

    #[test]
    fn test_parse_ss() {
        let link = String::from("ss://YWVzLTEyOC1nY206ZDljNTc3MzI4ZmIzNDlmZQ==@120.232.73.68:40676#%F0%9F%87%AD%F0%9F%87%B0HK");
        let result = SS::from_link(link.clone());
        assert!(result.is_ok());
        let proxy = result.unwrap();
        assert_eq!(proxy.name, "🇭🇰HK");
        assert_eq!(proxy.server, "120.232.73.68");
        assert_eq!(proxy.port, 40676);
        assert_eq!(proxy.password, "d9c577328fb349fe");
        assert_eq!(proxy.cipher, "aes-128-gcm");
        assert_eq!(proxy.to_link(), link)
    }

    #[test]
    fn test_ss2() {
        let link = String::from("ss://Y2hhY2hhMjAtaWV0ZjpIdVRhb0Nsb3Vk@cm1-hk.hutaonode3.top:12452?plugin=obfs-local;mode%3Dwebsocket#%E9%A6%99%E6%B8%AF%40vpnhat");
        let result = SS::from_link(link.clone()).unwrap();
        assert!(result.plugin.is_some());
        assert_eq!(result.to_link(), link);
    }

    #[test]
    fn test_ss3() {
        let link = String::from("ss://cmM0LW1kNToydnpobzU=@120.241.144.101:2410?plugin=obfs-local;obfs%3Dhttp;obfs-host%3D89c19109670.microsoft.com#%E9%A6%99%E6%B8%AFAkari-P");
        let ss1 = SS::from_link(link.clone()).unwrap();
        assert_eq!(ss1.cipher, "rc4-md5");
        assert_eq!(ss1.password, "2vzho5");
        assert_eq!(ss1.server, "120.241.144.101");
        assert_eq!(ss1.port, 2410);
        assert_eq!(ss1.plugin, Some("obfs-local".to_string()));
        let mut map = HashMap::new();
        map.insert("obfs".to_string(), "http".to_string());
        map.insert("obfs-host".to_string(), "89c19109670.microsoft.com".to_string());
        assert_eq!(ss1.plugin_opts, Some(map));

        let b_link = ss1.to_link();
        let ss2 = SS::from_link(b_link.clone()).unwrap();
        assert_eq!(ss2.cipher, "rc4-md5");
        assert_eq!(ss2.password, "2vzho5");
        assert_eq!(ss2.server, "120.241.144.101");
        assert_eq!(ss2.port, 2410);
        assert_eq!(ss2.plugin, Some("obfs-local".to_string()));
        let mut map = HashMap::new();
        map.insert("obfs".to_string(), "http".to_string());
        map.insert("obfs-host".to_string(), "89c19109670.microsoft.com".to_string());
        assert_eq!(ss2.plugin_opts, Some(map));
    }
}