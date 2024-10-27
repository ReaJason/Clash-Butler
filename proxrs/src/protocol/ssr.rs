use crate::base64::base64decode;
use crate::protocol::deserialize_u16_or_string;
use crate::protocol::{ProxyAdapter, UnsupportedLinkError};
use serde::{Deserialize, Serialize};
use serde_json::Error;
use std::any::Any;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

#[derive(Deserialize, Debug, Serialize, Eq, Clone)]
pub struct Ssr {
    pub name: String,
    pub server: String,
    #[serde(deserialize_with = "deserialize_u16_or_string")]
    pub port: u16,
    pub password: String,
    pub cipher: String,
    pub obfs: String,
    #[serde(skip_serializing_if = "Option::is_none", rename = "obfs-param")]
    pub obfs_param: Option<String>,
    pub protocol: String,
    #[serde(skip_serializing_if = "Option::is_none", rename = "protocol-param")]
    pub protocol_param: Option<String>,
}

impl PartialEq for Ssr {
    fn eq(&self, other: &Self) -> bool {
        self.server == other.server && self.port == other.port && self.password == other.password
    }
}

impl ProxyAdapter for Ssr {
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
        // ssr://dmlwLmJhc2ljbm9kZS5ob3N0OjExODQ1OmF1dGhfYWVzMTI4X3NoYTE6Y2hhY2hhMjAtaWV0Zjp0bHMxLjJfdGlja2V0X2F1dGg6Um1oaVpUQjYvP3JlbWFya3M9VUhKdkxlbW1tZWE0cnlCSVMwZmt1S2psaGFqb3A2UHBsSUhrdUtoQk1nPT0mb2Jmc3BhcmFtPU5tWTBNV0l5TkM1dGFXTnliM052Wm5RdVkyOXQmcHJvdG9wYXJhbT1NalE2VTNCWlZYUlFaVXBaYUZKck5FWlhRdz09
        let url: &str = &link[6..];

        // vip.basicnode.host:11845:auth_aes128_sha1:chacha20-ietf:tls1.2_ticket_auth:RmhiZTB6/?remarks=UHJvLemmmea4ryBIS0fkuKjlhajop6PplIHkuKhBMg==&obfsparam=NmY0MWIyNC5taWNyb3NvZnQuY29t&protoparam=MjQ6U3BZVXRQZUpZaFJrNEZXQw==
        let url = base64decode(url);
        let parts: Vec<&str> = url.split("/?").collect();

        let params = parts[1];
        let mut params_map: HashMap<&str, String> = HashMap::new();
        for param in params.split("&") {
            if let Some((key, value)) = param.split_once('=') {
                let value = base64decode(&value.parse::<String>().unwrap());
                params_map.insert(key, value);
            }
        }
        // vip.basicnode.host:11845:auth_aes128_sha1:chacha20-ietf:tls1.2_ticket_auth:RmhiZTB6
        // &server, &port, &protocol, &method, &obfs, &password
        let url = parts[0];
        let values: Vec<&str> = url.split(":").collect();
        let server = String::from(values[0]);
        let port = values[1].parse::<u16>().unwrap();
        let protocol = String::from(values[2]);
        let cipher = String::from(values[3]);
        let obfs = String::from(values[4]);
        let password = base64decode(values[5]);

        let mut name = String::from("");
        if let Some(result) = params_map.get("remarks") {
            name = result.clone();
        }

        Ok(Ssr {
            name,
            server,
            port,
            password,
            cipher,
            obfs,
            obfs_param: params_map.get("obfsparam").cloned(),
            protocol,
            protocol_param: params_map.get("protoparam").cloned(),
        })
    }

    fn to_json(&self) -> Result<String, Error> {
        serde_json::to_string(self)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn eq(&self, other: &dyn ProxyAdapter) -> bool {
        if let Some(other) = other.as_any().downcast_ref::<Ssr>() {
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
    fn test_parse_ssr() {
        let link = String::from("ssr://dmlwLmJhc2ljbm9kZS5ob3N0OjExODQ1OmF1dGhfYWVzMTI4X3NoYTE6Y2hhY2hhMjAtaWV0Zjp0bHMxLjJfdGlja2V0X2F1dGg6Um1oaVpUQjYvP3JlbWFya3M9VUhKdkxlbW1tZWE0cnlCSVMwZmt1S2psaGFqb3A2UHBsSUhrdUtoQk1nPT0mb2Jmc3BhcmFtPU5tWTBNV0l5TkM1dGFXTnliM052Wm5RdVkyOXQmcHJvdG9wYXJhbT1NalE2VTNCWlZYUlFaVXBaYUZKck5FWlhRdz09");
        let ssr = Ssr::from_link(link).unwrap();
        assert_eq!(ssr.server, "vip.basicnode.host");
        assert_eq!(ssr.port, 11845);
        assert_eq!(ssr.password, "Fhbe0z");
        assert_eq!(ssr.cipher, "chacha20-ietf");
        assert_eq!(ssr.obfs, "tls1.2_ticket_auth");
        assert_eq!(ssr.obfs_param, Some("6f41b24.microsoft.com".to_string()));
        assert_eq!(ssr.protocol, "auth_aes128_sha1");
        assert_eq!(ssr.protocol_param, Some("24:SpYUtPeJYhRk4FWC".to_string()));
        println!("{}", ssr.to_json().unwrap());
    }

    #[test]
    fn test_parse_ssr2() {
        let link = String::from("ssr://dXMtYW0zLmVxbm9kZS5uZXQ6ODA4MTpvcmlnaW46YWVzLTI1Ni1jZmI6dGxzMS4yX3RpY2tldF9hdXRoOlptOTFPRTFDUjJscS8/b2Jmc3BhcmFtPSZwcm90b3BhcmFtPSZyZW1hcmtzPXNzcl9tZXRhXzExJnByb3RvcGFyYW09Jm9iZnNwYXJhbT0=");
        let ssr = Ssr::from_link(link).unwrap();
        assert_eq!(ssr.server, "us-am3.eqnode.net");
        assert_eq!(ssr.port, 8081);
        assert_eq!(ssr.password, "fou8MBGij");
        assert_eq!(ssr.cipher, "aes-256-cfb");
        assert_eq!(ssr.obfs, "tls1.2_ticket_auth");
        assert_eq!(ssr.obfs_param, Some("".to_string()));
        assert_eq!(ssr.protocol, "origin");
        assert_eq!(ssr.protocol_param, Some("".to_string()));
        println!("{}", ssr.to_json().unwrap());
    }
}
