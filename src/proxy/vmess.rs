use crate::proxy::deserialize_u16_or_string;
use crate::proxy::{base64decode, base64encode, ProxyAdapter, UnsupportedLinkError, WSOptions};
use serde::{Deserialize, Serialize};
use serde_json::Error;
use std::any::Any;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

#[derive(Deserialize, Debug, Serialize, Eq, Clone)]
pub struct Vmess {
    name: String,
    server: String,
    #[serde(deserialize_with = "deserialize_u16_or_string")]
    port: u16,
    uuid: String,
    #[serde(deserialize_with = "deserialize_u16_or_string", rename = "alterId")]
    alter_id: u16,
    cipher: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    tls: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    udp: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    servername: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    fingerprint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    network: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "skip-cert-verify")]
    skip_cert_verify: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "ws-opts")]
    ws_opts: Option<WSOptions>,
}

impl PartialEq for Vmess {
    fn eq(&self, other: &Self) -> bool {
        self.server == other.server
            && self.port == other.port
            && self.uuid == other.uuid
    }
}

#[derive(Deserialize, Debug, Serialize)]
pub struct VmessProtocol {
    pub v: String,
    pub ps: String,
    pub add: String,
    pub port: u16,
    pub id: String,
    pub aid: u16,
    pub scy: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub net: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tls: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sni: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fp: Option<String>,
}

impl ProxyAdapter for Vmess {
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
        let mut host = None;
        let mut path = None;
        let net = self.network.clone();

        if net.is_some_and(|s| s == "ws") {
            let ws_opts = self.ws_opts.clone();
            if let Some(opts) = ws_opts {
                path = opts.path.clone();
                if let Some(headers) = opts.headers {
                    host = headers.get("host").cloned();
                }
            }
        }

        let vmess = VmessProtocol {
            v: "2".to_string(),
            ps: self.name.clone(),
            add: self.server.clone(),
            port: self.port,
            id: self.uuid.clone(),
            aid: self.alter_id,
            scy: self.cipher.clone(),
            net: self.network.clone(),
            host,
            path,
            tls: self.tls,
            sni: self.servername.clone(),
            fp: self.fingerprint.clone(),
        };
        "vmess://".to_string() + &*base64encode(serde_json::to_string(&vmess).unwrap())
    }

    fn from_link(link: String) -> Result<Self, UnsupportedLinkError>
    where
        Self: Sized,
    {
        let url = &link[8..];
        if let Ok(data) = base64decode(url) {
            let parsed: serde_json::Value = serde_json::from_str(&data).unwrap();

            let name = String::from(parsed["ps"].as_str().unwrap());
            let server = parsed["add"].as_str().unwrap().to_string();
            let alter_id = parsed["aid"].as_str().map_or_else(
                || parsed["aid"].as_u64().unwrap() as u16,
                |s| s.parse::<u16>().unwrap(),
            );
            let uuid = parsed["id"].as_str().unwrap().to_string();
            let port = parsed["port"].as_str().map_or_else(
                || parsed["port"].as_u64().unwrap() as u16,
                |s| s.parse::<u16>().unwrap(),
            );

            let mut network = parsed["net"].as_str().map(|s| s.to_string());
            let mut ws_opts = None;

            // parse ws-opts
            if network.as_deref().is_some_and(|s| s == "ws") {
                let path = parsed["path"].as_str().map(|s| s.to_string());
                let mut headers = HashMap::new();
                if let Some(host) = parsed["host"].as_str() {
                    headers.insert(String::from("host"), host.to_string());
                }
                ws_opts = Some(WSOptions {
                    path,
                    headers: Some(headers),
                });
            }

            if let Some(net) = network.as_deref() {
                if net == "quic" || net == "http" || net == "grpc" {
                    return Err(UnsupportedLinkError {
                        message: format!("vmess not suitable for network type {}", net),
                    });
                }

                if net.is_empty() {
                    network = None;
                }
            }

            let servername = parsed["sni"].as_str().map(|s| s.to_string());
            let udp = parsed["udp"].as_str().map(|s| s.parse::<bool>().unwrap_or(true));
            let tls = parsed["tls"].as_str().map(|s| s.parse::<bool>().unwrap_or(false));
            Ok(Vmess {
                name,
                server,
                port,
                uuid,
                alter_id,
                cipher: "auto".to_string(),
                tls,
                udp,
                servername,
                fingerprint: Some(String::from("chrome")),
                network,
                skip_cert_verify: Some(true),
                ws_opts,
            })
        } else {
            Err(UnsupportedLinkError {
                message: format!("Unsupported link format, base64 decode error: {}", link),
            })
        }
    }

    fn to_json(&self) -> Result<String, Error> {
        serde_json::to_string(self)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn eq(&self, other: &dyn ProxyAdapter) -> bool {
        if let Some(other) = other.as_any().downcast_ref::<Vmess>() {
            self == other
        } else {
            false
        }
    }

    fn hash(&self, mut state: &mut dyn Hasher) {
        self.server.hash(&mut state);
        self.port.hash(&mut state);
        self.uuid.hash(&mut state);
    }
}


#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse_vmess() {
        let link = String::from("vmess://eyJ2IjoiMiIsInBzIjoiXHU1MmEwXHU2MmZmXHU1OTI3IDAzIFx1OWFkOFx1OTAxZlx1ZmYwODAuMVx1NTAwZFx1NmQ0MVx1OTFjZlx1NmQ4OFx1ODAxN1x1ZmYwOSIsImFkZCI6ImNkbmNkbmNkbmNkbi43ODQ2NTQueHl6IiwicG9ydCI6IjIwNTIiLCJpZCI6IjNlYTU3OGM2LTFlYWEtNGUxNS1iZmUxLTlmNzU3YjU4ZThmMiIsImFpZCI6IjAiLCJuZXQiOiJ3cyIsInR5cGUiOiJub25lIiwiaG9zdCI6ImNhLWNmY2RuLmFpa3VuYXBwLmNvbSIsInBhdGgiOiJcL2luZGV4P2VkPTIwNDgiLCJ0bHMiOiIifQ==");
        let vmess = Vmess::from_link(link).unwrap();
        assert_eq!(vmess.server, "cdncdncdncdn.784654.xyz");
        assert_eq!(vmess.port, 2052);
        assert_eq!(vmess.uuid, "3ea578c6-1eaa-4e15-bfe1-9f757b58e8f2");
        assert_eq!(vmess.alter_id, 0);
        assert_eq!(vmess.network, Some("ws".to_string()));
        assert!(vmess.ws_opts.is_some());

        let link = String::from("vmess://eyJ2IjoiMiIsInBzIjoiQHZwbnBvb2wiLCJhZGQiOiJrci5haWt1bmFwcC5jb20iLCJwb3J0IjoyMDAwNiwiaWQiOiIyMTM2ZGM2Yy01ZmQ0LTRiZmQtODhhMS0yYWVlYTk4ODhmOGIiLCJhaWQiOjAsInNjeSI6ImF1dG8iLCJuZXQiOiIiLCJ0bHMiOiIifQ==");
        let vmess = Vmess::from_link(link).unwrap();
        assert_eq!(vmess.server, "kr.aikunapp.com");
        assert_eq!(vmess.port, 20006);
        assert_eq!(vmess.uuid, "2136dc6c-5fd4-4bfd-88a1-2aeea9888f8b");
        assert_eq!(vmess.alter_id, 0);
        assert_eq!(vmess.network, None);
        assert!(vmess.ws_opts.is_none());
    }
}