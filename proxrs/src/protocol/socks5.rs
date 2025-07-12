use std::any::Any;
use std::hash::Hash;
use std::hash::Hasher;

use serde::Deserialize;
use serde::Serialize;
use serde_json::Error;

use crate::protocol::deserialize_u16_or_string;
use crate::protocol::ProxyAdapter;
use crate::protocol::UnsupportedLinkError;

#[derive(Deserialize, Serialize, Debug, Eq, Clone)]
pub struct Socks5 {
    pub name: String,
    pub server: String,
    #[serde(deserialize_with = "deserialize_u16_or_string")]
    pub port: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tls: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub udp: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "skip-cert-verify")]
    pub skip_cert_verify: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fingerprint: Option<String>,
}

impl PartialEq for Socks5 {
    fn eq(&self, other: &Self) -> bool {
        self.server == other.server && self.port == other.port
    }
}

impl ProxyAdapter for Socks5 {
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
        if let Some(other) = other.as_any().downcast_ref::<Socks5>() {
            self == other
        } else {
            false
        }
    }

    fn hash(&self, mut state: &mut dyn Hasher) {
        self.server.hash(&mut state);
        self.password.hash(&mut state);
    }
}
