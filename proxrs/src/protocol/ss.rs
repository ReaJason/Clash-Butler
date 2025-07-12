use std::any::Any;
use std::collections::HashMap;
use std::hash::Hash;
use std::hash::Hasher;

use serde::Deserialize;
use serde::Serialize;
use serde_json::Error;
use serde_json::Value;

use crate::base64::base64decode;
use crate::base64::base64encode;
use crate::protocol::deserialize_u16_or_string;
use crate::protocol::ProxyAdapter;
use crate::protocol::UnsupportedLinkError;

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
    pub plugin_opts: Option<HashMap<String, Value>>,
}

impl PartialEq for SS {
    fn eq(&self, other: &Self) -> bool {
        self.server.eq(&other.server)
            && self.port.eq(&other.port)
            && self.password.eq(&other.password)
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
                        let v = key.clone() + "=" + value.as_str().unwrap();
                        urlencoding::encode(&v).into_owned()
                    })
                    .collect::<Vec<_>>()
                    .join(";");
                plugin.push_str(&str);
            }
            format!(
                "ss://{}@{}?{}#{}",
                cipher_pwd,
                server_port,
                plugin,
                urlencoding::encode(&self.name)
            )
        } else {
            format!(
                "ss://{}@{}#{}",
                cipher_pwd,
                server_port,
                urlencoding::encode(&self.name)
            )
        }
    }

    fn from_link(link: String) -> Result<Self, UnsupportedLinkError> {
        let url = base64decode(&link[5..]);
        // parse name
        let mut name = String::from("");
        let parts: Vec<&str> = url.split("#").collect();
        if parts.len() > 1 {
            name = urlencoding::decode(parts[1])
                .unwrap_or_default()
                .trim()
                .to_string();
        }

        // parse plugin
        let url = base64decode(parts[0]);
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
                    let mut map: HashMap<String, Value> = HashMap::new();
                    plugin_params[1..].iter().for_each(|param| {
                        let value = urlencoding::decode(param)
                            .unwrap_or_default()
                            .trim()
                            .to_string();
                        let kvs = value.split("=").collect::<Vec<_>>();
                        if kvs.len() == 2 {
                            map.insert(kvs[0].to_string(), kvs[1].into());
                        }
                    });
                    plugin_opts = Some(map);
                }
            }
        }

        // parse server port
        let url = parts[0];
        let secret_server_port_parts: Vec<&str> = url.split("@").collect();

        let secret = base64decode(secret_server_port_parts[0]);
        let cipher_pwd_parts: Vec<&str> = secret.splitn(2, ":").collect();
        let cipher = cipher_pwd_parts[0].parse().unwrap();
        let password = cipher_pwd_parts[1].parse().unwrap();

        let server_port = secret_server_port_parts[1];
        let server_port_parts: Vec<&str> = server_port.split(":").collect();
        let server = server_port_parts[0].parse::<String>().unwrap();
        let port = server_port_parts[1].parse::<u16>().unwrap();

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
    fn test_parse_base64_ss() {
        let link = String::from(
            "ss://YWVzLTI1Ni1nY206UTFHVVo3VkRQWk9BU0M5SEAxMjAuMjQxLjQ1LjUwOjE3MDAxI1VTLTAx",
        );
        let result = SS::from_link(link.clone()).unwrap();
        assert_eq!("Q1GUZ7VDPZOASC9H", result.password);
        assert_eq!("aes-256-gcm", result.cipher);
    }

    #[test]
    fn test_parse_base64_ss1() {
        let link = String::from(
            "ss://MjAyMi1ibGFrZTMtYWVzLTI1Ni1nY206emtWV2lPU1o4OEVnZi9LSlE1azFlWFRZUFNMNXhZWEZ6OTFPanBFRWE1UT06dzZLQTFFYkNrM2hpdWJQZWlMMktkUUJjcG9kbUl3c1VlcDJBLzFVd3hLbz1AYXdzMS5pb2xvZnQubWU6NDg1Njc#%F0%9F%87%AF%F0%9F%87%B5%20AWS",
        );
        let result = SS::from_link(link.clone()).unwrap();
        assert_eq!("zkVWiOSZ88Egf/KJQ5k1eXTYPSL5xYXFz91OjpEEa5Q=:w6KA1EbCk3hiubPeiL2KdQBcpodmIwsUep2A/1UwxKo=", result.password);
        assert_eq!("2022-blake3-aes-256-gcm", result.cipher);
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
        let mut map = HashMap::<String, Value>::new();
        map.insert("obfs".to_string(), "http".into());
        map.insert("obfs-host".into(), "89c19109670.microsoft.com".into());
        assert_eq!(ss1.plugin_opts, Some(map));

        let b_link = ss1.to_link();
        let ss2 = SS::from_link(b_link.clone()).unwrap();
        assert_eq!(ss2.cipher, "rc4-md5");
        assert_eq!(ss2.password, "2vzho5");
        assert_eq!(ss2.server, "120.241.144.101");
        assert_eq!(ss2.port, 2410);
        assert_eq!(ss2.plugin, Some("obfs-local".to_string()));
        let mut map = HashMap::<String, Value>::new();
        map.insert("obfs".to_string(), "http".into());
        map.insert("obfs-host".into(), "89c19109670.microsoft.com".into());
        assert_eq!(ss2.plugin_opts, Some(map));
    }

    #[test]
    fn test_ss_json() {
        let json_data = r#"
    {
        "name":"hello",
        "cipher": "aes-256-gcm",
        "type":"ss",
        "password":"941cbc4237e0",
        "server":"121.127.231.239",
        "port": 636,
        "plugin": "v2ray-plugin",
        "plugin-opts": {
            "host": "example.com",
            "mode": "websocket",
            "mux": true,
            "port": 443,
            "tls": true,
            "skip-cert-verify": false
        }
    }
    "#;

        match serde_json::from_str::<SS>(json_data) {
            Ok(ss) => {
                println!("{:?}", ss);
                assert!(ss.plugin_opts.is_some());
            }
            Err(e) => {
                println!("{}", e);
            }
        };
    }
}
