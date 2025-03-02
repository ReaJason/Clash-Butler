use std::any::Any;
use std::collections::HashMap;
use std::hash::Hash;
use std::hash::Hasher;

use serde::Deserialize;
use serde::Serialize;
use serde_json::Error;

use crate::base64::base64decode;
use crate::base64::base64encode;
use crate::protocol::deserialize_u16_or_string;
use crate::protocol::GrpcOptions;
use crate::protocol::ProxyAdapter;
use crate::protocol::RealtyOptions;
use crate::protocol::UnsupportedLinkError;
use crate::protocol::WSOptions;

#[derive(Deserialize, Debug, Serialize, Eq, Clone)]
pub struct Vmess {
    pub name: String,
    pub server: String,
    #[serde(deserialize_with = "deserialize_u16_or_string")]
    pub port: u16,
    pub uuid: String,
    #[serde(deserialize_with = "deserialize_u16_or_string", rename = "alterId")]
    pub alter_id: u16,
    pub cipher: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alpn: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tls: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub udp: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub servername: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fingerprint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "skip-cert-verify")]
    pub skip_cert_verify: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "ws-opts")]
    pub ws_opts: Option<WSOptions>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "grpc-opts")]
    pub grpc_opts: Option<GrpcOptions>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "reality-opts")]
    pub realty_opts: Option<RealtyOptions>,
}

impl PartialEq for Vmess {
    fn eq(&self, other: &Self) -> bool {
        self.server == other.server && self.port == other.port && self.uuid == other.uuid
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
    pub alpn: Option<String>,
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

        let mut alpn = None;

        if self.alpn.is_some() {
            alpn = Some(self.alpn.clone().unwrap().join(","))
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
            alpn,
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
        let url = base64decode(&link[8..]);
        match serde_json::from_str::<serde_json::Value>(&url) {
            Ok(parsed) => {
                let name = String::from(parsed["ps"].as_str().unwrap_or_default());
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

                let mut alpn = None;
                if let Some(p) = parsed["alpn"].as_str() {
                    alpn = Some(p.split(",").map(|s| s.to_string()).collect());
                }

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

                let mut grpc_opts = None;

                // parse grpc sni
                if network.as_deref().is_some_and(|s| s == "grpc") {
                    let sni = parsed["sni"].as_str().map(|s| s.to_string());
                    grpc_opts = Some(GrpcOptions {
                        grpc_service_name: sni,
                    })
                }

                if let Some(net) = network.as_deref() {
                    if net == "quic" || net == "http" {
                        return Err(UnsupportedLinkError {
                            message: format!("vmess not suitable for network type {}", net),
                        });
                    }

                    if net.is_empty() {
                        network = None;
                    }
                }

                let servername = parsed["sni"].as_str().map(|s| s.to_string());
                let udp = parsed["udp"]
                    .as_str()
                    .map(|s| s.parse::<bool>().unwrap_or(true));
                let tls = parsed["tls"]
                    .as_str()
                    .map(|s| s.parse::<bool>().unwrap_or(false));
                Ok(Vmess {
                    name,
                    server,
                    port,
                    uuid,
                    alter_id,
                    cipher: "auto".to_string(),
                    tls,
                    udp,
                    alpn,
                    servername,
                    fingerprint: Some(String::from("chrome")),
                    network,
                    skip_cert_verify: Some(true),
                    ws_opts,
                    grpc_opts,
                    realty_opts: None,
                })
            }
            Err(_) => {
                // parse params
                let parts: Vec<&str> = url.split("?").collect();
                let params = parts[1];
                let mut params_map: HashMap<&str, String> = HashMap::new();
                for param in params.split("&") {
                    if let Some((key, value)) = param.split_once('=') {
                        let value = value.parse::<String>().unwrap();
                        params_map.insert(key, value);
                    }
                }
                let alter_id = params_map.get("alterId").unwrap().parse::<u16>().unwrap();
                let name = urlencoding::decode(params_map.get("remarks").unwrap()).unwrap().to_string();

                // parse server port
                let url = base64decode(parts[0]);
                let secret_server_port_parts: Vec<&str> = url.split("@").collect();

                let secret = base64decode(secret_server_port_parts[0]);
                let cipher_pwd_parts: Vec<&str> = secret.splitn(2, ":").collect();
                let cipher = cipher_pwd_parts[0].parse().unwrap();
                let uuid = cipher_pwd_parts[1].parse().unwrap();

                let server_port = secret_server_port_parts[1];
                let server_port_parts: Vec<&str> = server_port.split(":").collect();
                let server = server_port_parts[0].parse::<String>().unwrap();
                let port = server_port_parts[1].parse::<u16>().unwrap();
                Ok(Vmess{
                    name,
                    server,
                    port,
                    uuid,
                    alter_id,
                    cipher,
                    alpn: None,
                    tls: None,
                    udp: None,
                    servername: None,
                    fingerprint: None,
                    network: None,
                    skip_cert_verify: Some(true),
                    ws_opts: None,
                    grpc_opts: None,
                    realty_opts: None,
                })
            }
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
        self.network.hash(&mut state);
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse_vmess() {
        let link = String::from("vmess://eyJ2IjoiMiIsInBzIjoiQHZwbnBvb2wiLCJhZGQiOiJrci5haWt1bmFwcC5jb20iLCJwb3J0IjoyMDAwNiwiaWQiOiIyMTM2ZGM2Yy01ZmQ0LTRiZmQtODhhMS0yYWVlYTk4ODhmOGIiLCJhaWQiOjAsInNjeSI6ImF1dG8iLCJuZXQiOiIiLCJ0bHMiOiIifQ==");
        let vmess = Vmess::from_link(link).unwrap();
        assert_eq!(vmess.server, "kr.aikunapp.com");
        assert_eq!(vmess.port, 20006);
        assert_eq!(vmess.uuid, "2136dc6c-5fd4-4bfd-88a1-2aeea9888f8b");
        assert_eq!(vmess.alter_id, 0);
        assert_eq!(vmess.network, None);
        assert!(vmess.ws_opts.is_none());
    }

    #[test]
    fn test_parse_ss_vmess() {
        let link = String::from("vmess://YXV0bzoyMTc5ZjE3OS01MzYwLTRiYzAtOWJjYS1mZmQ1NDAzNjNlNGFAMjAzLjY2LjE0LjQ4OjQ1MTE2?remarks=%F0%9F%87%B9%F0%9F%87%BC%20%E6%B8%AF%E4%BB%94&obfs=none&alterId=0");
        let vmess = Vmess::from_link(link).unwrap();
        assert_eq!(vmess.server, "203.66.14.48");
        assert_eq!(vmess.port, 45116);
        assert_eq!(vmess.uuid, "2179f179-5360-4bc0-9bca-ffd540363e4a");
        assert_eq!(vmess.cipher, "auto");
        assert_eq!(vmess.alter_id, 0);
        assert_eq!(vmess.network, None);
        assert!(vmess.ws_opts.is_none());
    }

    #[test]
    fn test_parse_ws_vmess() {
        let link = String::from("vmess://eyJ2IjoiMiIsInBzIjoiXHU1MmEwXHU2MmZmXHU1OTI3IDAzIFx1OWFkOFx1OTAxZlx1ZmYwODAuMVx1NTAwZFx1NmQ0MVx1OTFjZlx1NmQ4OFx1ODAxN1x1ZmYwOSIsImFkZCI6ImNkbmNkbmNkbmNkbi43ODQ2NTQueHl6IiwicG9ydCI6IjIwNTIiLCJpZCI6IjNlYTU3OGM2LTFlYWEtNGUxNS1iZmUxLTlmNzU3YjU4ZThmMiIsImFpZCI6IjAiLCJuZXQiOiJ3cyIsInR5cGUiOiJub25lIiwiaG9zdCI6ImNhLWNmY2RuLmFpa3VuYXBwLmNvbSIsInBhdGgiOiJcL2luZGV4P2VkPTIwNDgiLCJ0bHMiOiIifQ==");
        let vmess = Vmess::from_link(link).unwrap();
        assert_eq!(vmess.server, "cdncdncdncdn.784654.xyz");
        assert_eq!(vmess.port, 2052);
        assert_eq!(vmess.uuid, "3ea578c6-1eaa-4e15-bfe1-9f757b58e8f2");
        assert_eq!(vmess.alter_id, 0);
        assert_eq!(vmess.network, Some("ws".to_string()));
        assert!(vmess.ws_opts.is_some());
    }

    #[test]
    fn test_parse_grpc_vmess() {
        let link = String::from("vmess://eyJ2IjoiMiIsInBzIjoiXHU5MDgwXHU4YmY3XHU2NWIwXHU3NTI4XHU2MjM3NjAlXHU4ZmQ0XHU1MjI5IiwiYWRkIjoiZGVmYXVsdC42NTNlYmVlYi01ZjYwLTRiZTUtOTU4ZC03YmY0ODM5Y2RjY2QuZWY2NjE2ZmQtNWIwNi00ODJmLTlkNjQtMTgzNzQ1NjU5Y2JmLmJ5dGVwcml2YXRlbGluay5jb20iLCJwb3J0IjoiNDQzIiwiaWQiOiJhNDQzMDZkNS0zMzQzLTQ0MDUtYTA4Yy0yZDU0NmE1N2QzYjgiLCJhaWQiOiIwIiwibmV0IjoiZ3JwYyIsInR5cGUiOiJub25lIiwiaG9zdCI6IiIsInBhdGgiOiIxMjMwNiIsInRscyI6InRscyIsInNuaSI6ImNkbjEuMTAzOTIub25saW5lIn0=");
        let vmess = Vmess::from_link(link).unwrap();
        assert_eq!(Some("grpc".to_string()), vmess.network);
        assert_eq!(
            Some(GrpcOptions {
                grpc_service_name: Some("cdn1.10392.online".to_string())
            }),
            vmess.grpc_opts
        );
    }
}
