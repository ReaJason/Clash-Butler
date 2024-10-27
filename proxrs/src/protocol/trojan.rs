use std::any::Any;
use std::collections::HashMap;
use std::hash::Hash;
use std::hash::Hasher;

use serde::Deserialize;
use serde::Serialize;
use serde_json::Error;

use crate::protocol::deserialize_u16_or_string;
use crate::protocol::ProxyAdapter;
use crate::protocol::UnsupportedLinkError;

#[derive(Deserialize, Debug, Serialize, Eq, Clone)]
pub struct Trojan {
    pub name: String,
    pub server: String,
    #[serde(deserialize_with = "deserialize_u16_or_string")]
    pub port: u16,
    pub password: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sni: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "skip-cert-verify")]
    pub skip_cert_verify: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network: Option<String>,
}

impl PartialEq for Trojan {
    fn eq(&self, other: &Self) -> bool {
        self.server == other.server && self.port == other.port && self.password == other.password
    }
}

impl ProxyAdapter for Trojan {
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

    fn from_link(link: String) -> Result<Self, UnsupportedLinkError>
    where
        Self: Sized,
    {
        // trojan://4fee57cc-ee15-4800-888f-3493f7b261f2@hk1.ee2c9087-71b0-70af-7924-09d714b25b96.
        // 6df03129.the-best-airport.com:443?type=tcp&sni=new.download.the-best-airport.com&
        // allowInsecure=1#%F0%9F%87%AD%F0%9F%87%B0%E9%A6%99%E6%B8%AF%2001%20%7C%20%E4%B8%93%E7%BA%
        // BF%0D
        let mut url = &link[9..];

        let mut name = String::from("");
        if let Some((v1, v2)) = url.rsplit_once("#") {
            url = v1;
            name = urlencoding::decode(v2).unwrap().to_string();
        }
        // b7c0a9b4-0b85-4e93-921e-63bef702172b@111.38.53.159:41001
        // 4fee57cc-ee15-4800-888f-3493f7b261f2@hk1.ee2c9087-71b0-70af-7924-09d714b25b96.6df03129.
        // the-best-airport.com:443?type=tcp&sni=new.download.the-best-airport.com&allowInsecure=1
        let parts: Vec<&str> = url.split("?").collect();
        let mut network = None;
        let mut sni = None;
        let mut skip_cert_verify = None;
        if parts.len() > 1 {
            let params = parts[1];
            let mut params_map: HashMap<&str, String> = HashMap::new();
            for param in params.split("&") {
                if let Some((key, value)) = param.split_once('=') {
                    let value = value.parse::<String>().unwrap();
                    params_map.insert(key, value);
                }
            }
            network = params_map.get("type").cloned();
            sni = params_map.get("sni").cloned();
            skip_cert_verify = params_map.get("allowInsecure").map(|value| value == "1");
        }

        let url = parts[0];
        // 4fee57cc-ee15-4800-888f-3493f7b261f2@hk1.ee2c9087-71b0-70af-7924-09d714b25b96.6df03129.
        // the-best-airport.com:443
        let parts: Vec<&str> = url.split("@").collect();
        let password = String::from(parts[0]);

        let parts: Vec<&str> = parts[1].split(":").collect();
        let server = String::from(parts[0]);
        let port = parts[1].parse::<u16>().unwrap();

        Ok(Trojan {
            name,
            server,
            port,
            password,
            sni,
            skip_cert_verify,
            network,
        })
    }

    fn to_json(&self) -> Result<String, Error> {
        serde_json::to_string(self)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn eq(&self, other: &dyn ProxyAdapter) -> bool {
        if let Some(other) = other.as_any().downcast_ref::<Trojan>() {
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
    fn test_parse_trojan() {
        let link = String::from("trojan://4fee57cc-ee15-4800-888f-3493f7b261f2@hk1.ee2c9087-71b0-70af-7924-09d714b25b96.6df03129.the-best-airport.com:443?type=tcp&sni=new.download.the-best-airport.com&allowInsecure=1#%F0%9F%87%AD%F0%9F%87%B0%E9%A6%99%E6%B8%AF%2001%20%7C%20%E4%B8%93%E7%BA%BF%0D");
        let trojan = Trojan::from_link(link).unwrap();
        assert_eq!(
            trojan.server,
            "hk1.ee2c9087-71b0-70af-7924-09d714b25b96.6df03129.the-best-airport.com"
        );
        assert_eq!(trojan.port, 443);
        assert_eq!(trojan.password, "4fee57cc-ee15-4800-888f-3493f7b261f2");
        assert_eq!(
            trojan.sni,
            Some("new.download.the-best-airport.com".to_string())
        );
        assert_eq!(trojan.skip_cert_verify, Some(true));
        assert_eq!(trojan.network, Some("tcp".to_string()));
        println!("{:?}", trojan.to_json());
    }

    #[test]
    fn test_parse_trojan1() {
        let link = String::from("trojan://ed4f18fc-fdc9-4296-a69a-a2c908f9b09e@211.99.98.83:32039?security=tls&type=tcp&headerType=none#%F0%9F%87%A8%F0%9F%87%A6%20%E5%8A%A0%E6%8B%BF%E5%A4%A7-BGP");
        println!("{:?}", Trojan::from_link(link).unwrap().to_json());
    }
}
