use crate::proxy::deserialize_u16_or_string;
use crate::proxy::{ProxyAdapter, UnsupportedLinkError};
use serde::{Deserialize, Serialize};
use serde_json::Error;
use std::any::Any;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

#[derive(Deserialize, Serialize, Debug, Eq, Clone)]
pub struct Hysteria2 {
    pub name: String,
    pub server: String,
    pub password: String,
    #[serde(deserialize_with = "deserialize_u16_or_string")]
    pub port: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ports: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hop_interval: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub up: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub down: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub obfs: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub obfs_password: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sni: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skip_cert_verify: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fingerprint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "client-fingerprint")]
    pub client_fingerprint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alpn: Option<Vec<String>>,
}

impl PartialEq for Hysteria2 {
    fn eq(&self, other: &Self) -> bool {
        self.server == other.server
            && self.password == other.password
            && self.port == other.port
    }
}

impl ProxyAdapter for Hysteria2 {
    fn get_name(&self) -> &str {
        &self.name
    }

    fn set_name(&mut self, name: &str) {
        self.name = name.to_string();
    }

    fn get_server(&self) -> &str {
        &self.server
    }

    fn to_link(&self) -> String {
        todo!()
    }

    /*
        https://github.com/apernet/hysteria/blob/21ea2a024a5bd2d85b8c3e1350038fa178f0901b/app/cmd/client.go#L346
        hysteria2://auth@server:port/?insecure=1&sni=&obfs=&obfs-password=&pinSHA256=
     */
    fn from_link(link: String) -> Result<Self, UnsupportedLinkError>
    where
        Self: Sized,
    {
        // hysteria2://bfbe4deb-07c8-450b-945e-e3c7676ba5ed@163.123.192.167:50000/?insecure=1&sni=www.microsoft.com&mport=50000-50080#%E5%89%A9%E4%BD%99%E6%B5%81%E9%87%8F%EF%BC%9A163.97%20GB
        let url = &link[12..];
        let parts = url.split("#").collect::<Vec<_>>();
        let mut name = "".to_string();
        if parts.len() > 1 {
            name = urlencoding::decode(parts[1]).unwrap().to_string();
        }

        let url = parts[0];
        let mut parts = url.split("/?").collect::<Vec<_>>();
        if parts.len() == 1 {
            parts = url.split("?").collect::<Vec<_>>();
        }

        let params = parts[1];
        let mut params_map: HashMap<&str, String> = HashMap::new();
        for param in params.split("&") {
            if let Some((key, value)) = param.split_once('=') {
                let value = value.parse::<String>().unwrap();
                params_map.insert(key, value);
            }
        }

        let skip_cert_verify = params_map.get("insecure").is_some_and(|s| s == "1");
        let sni = params_map.get("sni").cloned();
        let up = params_map.get("up").cloned();
        let down = params_map.get("down").cloned();
        let mut alpn = None;
        if let Some(value) = params_map.get("alpn").cloned() {
            alpn = Some(value.split(",").collect::<Vec<_>>().into_iter().map(|s| s.to_string()).collect());
        }
        let obfs = params_map.get("obfs").cloned();
        let obfs_password = params_map.get("obfs-password").cloned();

        let url = parts[0];
        let parts: Vec<&str> = url.split("@").collect();
        let password = String::from(parts[0]);
        let parts: Vec<&str> = parts[1].split(":").collect();
        let server = String::from(parts[0]);
        let port;
        let mut ports = None;
        match parts[1].parse::<u16>() {
            Ok(p) => {
                port = p;
            }
            Err(_) => {
                let parts = parts[1].split(",").collect::<Vec<_>>();
                port = parts[0].parse::<u16>().unwrap();
                ports = Some(String::from(parts[1]));
            }
        }


        if name.is_empty() {
            name = server.clone() + port.to_string().as_str();
        }

        Ok(Hysteria2 {
            name,
            server,
            password,
            port,
            ports,
            alpn,
            hop_interval: None,
            up,
            down,
            obfs,
            obfs_password,
            sni,
            skip_cert_verify: Some(skip_cert_verify),
            client_fingerprint: Some("chrome".to_string()),
            fingerprint: None,
        })
    }

    fn to_json(&self) -> Result<String, Error> {
        serde_json::to_string(self)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn eq(&self, other: &dyn ProxyAdapter) -> bool {
        if let Some(other) = other.as_any().downcast_ref::<Hysteria2>() {
            self == other
        } else {
            false
        }
    }

    fn hash(&self, mut state: &mut dyn Hasher) {
        self.server.hash(&mut state);
        self.password.hash(&mut state);
        self.ports.hash(&mut state);
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse_vless() {
        let link = String::from("hysteria2://bfbe4deb-07c8-450b-945e-e3c7676ba5ed@163.123.192.167:50000/?insecure=1&sni=www.microsoft.com&mport=50000-50080#%E5%89%A9%E4%BD%99%E6%B5%81%E9%87%8F%EF%BC%9A163.97%20GB");
        let hysteria2 = Hysteria2::from_link(link).unwrap();
        assert_eq!(hysteria2.server, "163.123.192.167");
        assert_eq!(hysteria2.port, 50000);
        assert_eq!(hysteria2.ports, Some("50000-50080".to_string()));
        assert_eq!(hysteria2.password, "bfbe4deb-07c8-450b-945e-e3c7676ba5ed");
        assert_eq!(hysteria2.sni, Some("www.microsoft.com".to_string()));
        assert_eq!(hysteria2.skip_cert_verify, Some(true));
        assert_eq!(hysteria2.fingerprint, Some("chrome".to_string()));
        println!("{}", hysteria2.to_json().unwrap());
    }
}