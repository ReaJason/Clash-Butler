use std::any::Any;
use std::hash::Hash;
use std::hash::Hasher;

use serde::Deserialize;
use serde::Serialize;
use serde_json::Error;

use crate::protocol::ProxyAdapter;
use crate::protocol::UnsupportedLinkError;

#[derive(Deserialize, Serialize, Debug, Eq, Clone)]
pub struct Hysteria {
    pub name: String,
    pub server: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ports: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protocol: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "obfs-protocol")]
    pub obs_protocol: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub up: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "up-speed")]
    pub up_speed: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub down: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "down-speed")]
    pub down_speed: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "auth-str")]
    pub auth_str: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub obfs: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sni: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "skip-cert-verify")]
    pub skip_cert_verify: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fingerprint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alpn: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "resv-window-conn")]
    pub receive_windows_conn: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "resv-window")]
    pub receive_windows: Option<bool>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        rename = "disable-mtu-discovery"
    )]
    pub disable_mtu_discovery: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "fast-open")]
    pub fast_open: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "hop-interval")]
    pub hop_interval: Option<u16>,
}

impl PartialEq for Hysteria {
    fn eq(&self, other: &Self) -> bool {
        if self.port.is_some() {
            self.server == other.server && self.port == other.port
        } else {
            self.server == other.server && self.ports == other.ports
        }
    }
}

impl ProxyAdapter for Hysteria {
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

    fn from_link(_link: String) -> Result<Self, UnsupportedLinkError>
    where
        Self: Sized,
    {
        todo!()
    }

    fn to_json(&self) -> Result<String, Error> {
        serde_json::to_string(self)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn eq(&self, other: &dyn ProxyAdapter) -> bool {
        if let Some(other) = other.as_any().downcast_ref::<Hysteria>() {
            self == other
        } else {
            false
        }
    }

    fn hash(&self, mut state: &mut dyn Hasher) {
        self.server.hash(&mut state);
        if self.port.is_some() {
            self.port.hash(&mut state);
        } else {
            self.ports.hash(&mut state);
        }
    }
}
