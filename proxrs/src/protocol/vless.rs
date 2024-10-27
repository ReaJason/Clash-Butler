use std::any::Any;
use std::collections::HashMap;
use std::hash::Hash;
use std::hash::Hasher;

use serde::Deserialize;
use serde::Serialize;
use serde_json::Error;

use crate::protocol::deserialize_u16_or_string;
use crate::protocol::GrpcOptions;
use crate::protocol::ProxyAdapter;
use crate::protocol::RealtyOptions;
use crate::protocol::UnsupportedLinkError;
use crate::protocol::WSOptions;

#[derive(Deserialize, Debug, Serialize, Eq, Clone)]
pub struct Vless {
    name: String,
    server: String,
    #[serde(deserialize_with = "deserialize_u16_or_string")]
    port: u16,
    uuid: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    flow: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tls: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    udp: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "skip-cert-verify")]
    skip_cert_verify: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    fingerprint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    servername: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    network: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "ws-opts")]
    ws_opts: Option<WSOptions>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "reality-opts")]
    reality_opts: Option<RealtyOptions>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "grpc-opts")]
    grpc_opts: Option<GrpcOptions>,
}

impl PartialEq for Vless {
    fn eq(&self, other: &Self) -> bool {
        self.server == other.server && self.port == other.port && self.uuid == other.uuid
    }
}

impl ProxyAdapter for Vless {
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
        let url = &link[8..];
        let parts = url.split("#").collect::<Vec<_>>();
        let mut name = "".to_string();
        if parts.len() > 1 {
            name = urlencoding::decode(parts[1]).unwrap().to_string();
        }

        let url = parts[0];
        let parts = url.split("?").collect::<Vec<_>>();

        let params = parts[1];
        let mut params_map: HashMap<&str, String> = HashMap::new();
        for param in params.split("&") {
            if let Some((key, value)) = param.split_once('=') {
                let value = value.parse::<String>().unwrap();
                params_map.insert(key, value);
            }
        }

        let tls = params_map.get("security").is_some_and(|s| s == "tls");
        let network = params_map.get("type").cloned();
        let servername = params_map.get("sni").cloned();
        let flow = params_map.get("flow").cloned();
        let fingerprint = params_map.get("fp").cloned();
        let mut ws_opts = None;

        if network.as_deref().is_some_and(|s| s == "ws") {
            let mut headers = HashMap::new();
            if let Some(host) = params_map.get("host") {
                headers.insert(String::from("host"), host.to_string());
            }
            ws_opts = Some(WSOptions {
                path: params_map
                    .get("path")
                    .map(|s| urlencoding::decode(s).unwrap().to_string()),
                headers: Some(headers),
            })
        }

        let url = parts[0];
        let parts: Vec<&str> = url.split("@").collect();
        let uuid = String::from(parts[0]);
        let parts: Vec<&str> = parts[1].split(":").collect();
        let server = String::from(parts[0]);
        let port = parts[1].parse::<u16>().unwrap();

        if name.is_empty() {
            name = server.clone() + port.to_string().as_str();
        }

        Ok(Vless {
            name,
            server,
            port,
            uuid,
            flow,
            udp: Some(true),
            tls: Some(tls),
            skip_cert_verify: Some(true),
            fingerprint,
            servername,
            ws_opts,
            reality_opts: None,
            network,
            grpc_opts: None,
        })
    }

    fn to_json(&self) -> Result<String, Error> {
        serde_json::to_string(self)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn eq(&self, other: &dyn ProxyAdapter) -> bool {
        if let Some(other) = other.as_any().downcast_ref::<Vless>() {
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
    fn test_parse_vless() {
        // vless://2cd6ed0f-636e-4e6c-9449-5a263d7a0fa5@192.9.165.253:20001?encryption=none&
        // security=tls&sni=cfed.tgzdyz2.top&fp=random&type=ws&host=cfed.tgzdyz2.top&path=%2FTG%
        // 40ZDYZ2%3Fed%3D2560#TG%40ZDYZ2%20-%E6%BE%B3%E5%A4%A7%E5%88%A9%E4%BA%9A%F0%9F%87%A6%F0%9F%
        // 87%BA
        let link = String::from("vless://2cd6ed0f-636e-4e6c-9449-5a263d7a0fa5@192.9.165.253:20001?encryption=none&security=tls&sni=cfed.tgzdyz2.top&fp=random&type=ws&host=cfed.tgzdyz2.top&path=%2FTG%40ZDYZ2%3Fed%3D2560#TG%40ZDYZ2%20-%E6%BE%B3%E5%A4%A7%E5%88%A9%E4%BA%9A%F0%9F%87%A6%F0%9F%87%BA");
        let vless = Vless::from_link(link).unwrap();
        assert_eq!(vless.server, "192.9.165.253");
        assert_eq!(vless.port, 20001);
        assert_eq!(vless.tls, Some(true));
        assert_eq!(vless.uuid, "2cd6ed0f-636e-4e6c-9449-5a263d7a0fa5");
        assert_eq!(vless.servername, Some("cfed.tgzdyz2.top".to_string()));
        assert_eq!(vless.skip_cert_verify, Some(true));
        assert_eq!(vless.network, Some("ws".to_string()));
        let mut headers = HashMap::new();
        headers.insert("host".to_string(), "cfed.tgzdyz2.top".to_string());
        assert_eq!(
            vless.ws_opts,
            Some(WSOptions {
                path: Some("/TG@ZDYZ2?ed=2560".to_string()),
                headers: Some(headers),
            })
        );
        assert_eq!(vless.fingerprint, Some("random".to_string()));
        println!("{}", vless.to_json().unwrap());

        let new = Vless {
            name: "xixixi".to_string(),
            server: "192.9.165.253".to_string(),
            port: 20001,
            uuid: "2cd6ed0f-636e-4e6c-9449-5a263d7a0fa5".to_string(),
            tls: None,
            flow: None,
            udp: None,
            skip_cert_verify: None,
            fingerprint: None,
            servername: None,
            network: None,
            ws_opts: None,
            reality_opts: None,
            grpc_opts: None,
        };
        assert_eq!(new, vless);
    }

    #[test]
    fn test_parse_vless1() {
        let link = String::from("vless://bfbe4deb-07c8-450b-945e-e3c7676ba5ed@146.56.43.3:443?type=tcp&encryption=none&host=&path=&headerType=none&quicSecurity=none&serviceName=&mode=gun&security=tls&flow=xtls-rprx-vision&fp=safari&sni=djdownloadkr1.xn--4gq62f52gopi49k.com&pbk=&sid=#%F0%9F%87%B0%F0%9F%87%B7%E9%9F%A9%E5%9B%BD%E9%A6%96%E5%B0%942");
        let vless = Vless::from_link(link).unwrap();
        assert_eq!(vless.server, "146.56.43.3");
        assert_eq!(vless.port, 443);
        assert_eq!(vless.tls, Some(true));
        assert_eq!(vless.flow, Some("xtls-rprx-vision".to_string()));
        assert_eq!(vless.network, Some("tcp".to_string()));
        assert_eq!(vless.uuid, "bfbe4deb-07c8-450b-945e-e3c7676ba5ed");
        assert_eq!(
            vless.servername,
            Some("djdownloadkr1.xn--4gq62f52gopi49k.com".to_string())
        );
        assert_eq!(vless.fingerprint, Some("safari".to_string()));
        println!("{}", vless.to_json().unwrap());
    }

    // vless://b3524347-d27b-4d4a-8371-6cf837dea4d2@us1.helloco.xyz:60001?mode=multi&
    // security=reality&encryption=none&type=tcp&flow=xtls-rprx-vision&
    // pbk=Kyrdn7OhtL66JwSRScElBxoFSZLr5beafP4njt_Y_G0&sid=a3ffb25d&sni=python.org&
    // servername=python.org&spx=%2F&fp=qq#%E5%89%A9%E4%BD%99%E6%B5%81%E9%87%8F%EF%BC%9A510.48+GB
    // vless://b3524347-d27b-4d4a-8371-6cf837dea4d2@us1.helloco.xyz:60001?mode=multi&
    // security=reality&encryption=none&type=tcp&flow=xtls-rprx-vision&
    // pbk=Kyrdn7OhtL66JwSRScElBxoFSZLr5beafP4njt_Y_G0&sid=a3ffb25d&sni=python.org&
    // servername=python.org&spx=%2F&fp=safari#%E8%B7%9D%E7%A6%BB%E4%B8%8B%E6%AC%A1%E9%87%8D%E7%BD%
    // AE%E5%89%A9%E4%BD%99%EF%BC%9A29+%E5%A4%A9 vless://b3524347-d27b-4d4a-8371-6cf837dea4d2@
    // us1.helloco.xyz:60001?mode=multi&security=reality&encryption=none&type=tcp&
    // flow=xtls-rprx-vision&pbk=Kyrdn7OhtL66JwSRScElBxoFSZLr5beafP4njt_Y_G0&sid=a3ffb25d&
    // sni=python.org&servername=python.org&spx=%2F&fp=ios#%E5%A5%97%E9%A4%90%E5%88%B0%E6%9C%9F%EF%
    // BC%9A2024-10-16 vless://b3524347-d27b-4d4a-8371-6cf837dea4d2@us1.helloco.xyz:60001?
    // mode=multi&security=reality&encryption=none&type=tcp&flow=xtls-rprx-vision&
    // pbk=Kyrdn7OhtL66JwSRScElBxoFSZLr5beafP4njt_Y_G0&sid=a3ffb25d&sni=python.org&
    // servername=python.org&spx=%2F&fp=edge#United+States+01
    // vless://b3524347-d27b-4d4a-8371-6cf837dea4d2@ us2.helloco.xyz:60001?mode=multi&
    // security=reality&encryption=none&type=tcp& flow=xtls-rprx-vision&
    // pbk=Kyrdn7OhtL66JwSRScElBxoFSZLr5beafP4njt_Y_G0&sid=a3ffb25d& sni=python.org&
    // servername=python.org&spx=%2F&fp=qq#United+States+02
    // vless://b3524347-d27b-4d4a-8371-6cf837dea4d2@ us3.helloco.xyz:60001?mode=multi&
    // security=reality&encryption=none&type=tcp& flow=xtls-rprx-vision&
    // pbk=Kyrdn7OhtL66JwSRScElBxoFSZLr5beafP4njt_Y_G0&sid=a3ffb25d& sni=python.org&
    // servername=python.org&spx=%2F&fp=safari#United+States+03
    // vless://b3524347-d27b-4d4a-8371-6cf837dea4d2@ us4.helloco.xyz:60001?mode=multi&
    // security=reality&encryption=none&type=tcp& flow=xtls-rprx-vision&
    // pbk=Kyrdn7OhtL66JwSRScElBxoFSZLr5beafP4njt_Y_G0&sid=a3ffb25d& sni=python.org&
    // servername=python.org&spx=%2F&fp=firefox#United+States+04
    // vless://b3524347-d27b-4d4a-8371-6cf837dea4d2@ jp1.helloco.xyz:60001?mode=multi&
    // security=reality&encryption=none&type=tcp& flow=xtls-rprx-vision&
    // pbk=Kyrdn7OhtL66JwSRScElBxoFSZLr5beafP4njt_Y_G0&sid=a3ffb25d& sni=python.org&
    // servername=python.org&spx=%2F&fp=ios#Japan+01 vless://b3524347-d27b-4d4a-8371-6cf837dea4d2@
    // jp2.helloco.xyz:60001?mode=multi&security=reality&encryption=none&type=tcp&
    // flow=xtls-rprx-vision&pbk=Kyrdn7OhtL66JwSRScElBxoFSZLr5beafP4njt_Y_G0&sid=a3ffb25d&
    // sni=python.org&servername=python.org&spx=%2F&fp=edge#Japan+02
    // vless://b3524347-d27b-4d4a-8371-6cf837dea4d2@ jp3.helloco.xyz:60001?mode=multi&
    // security=reality&encryption=none&type=tcp& flow=xtls-rprx-vision&
    // pbk=Kyrdn7OhtL66JwSRScElBxoFSZLr5beafP4njt_Y_G0&sid=a3ffb25d& sni=python.org&
    // servername=python.org&spx=%2F&fp=safari#Japan+03
    // vless://b3524347-d27b-4d4a-8371-6cf837dea4d2@ jp4.helloco.xyz:60001?mode=multi&
    // security=reality&encryption=none&type=tcp& flow=xtls-rprx-vision&
    // pbk=Kyrdn7OhtL66JwSRScElBxoFSZLr5beafP4njt_Y_G0&sid=a3ffb25d& sni=python.org&
    // servername=python.org&spx=%2F&fp=qq#Japan+04 vless://b3524347-d27b-4d4a-8371-6cf837dea4d2@
    // kr1.helloco.xyz:60001?mode=multi&security=reality&encryption=none&type=tcp&
    // flow=xtls-rprx-vision&pbk=Kyrdn7OhtL66JwSRScElBxoFSZLr5beafP4njt_Y_G0&sid=a3ffb25d&
    // sni=python.org&servername=python.org&spx=%2F&fp=safari#Korea+01
    // vless://b3524347-d27b-4d4a-8371-6cf837dea4d2@ kr2.helloco.xyz:60001?mode=multi&
    // security=reality&encryption=none&type=tcp& flow=xtls-rprx-vision&
    // pbk=Kyrdn7OhtL66JwSRScElBxoFSZLr5beafP4njt_Y_G0&sid=a3ffb25d& sni=python.org&
    // servername=python.org&spx=%2F&fp=firefox#Korea+02
    // vless://b3524347-d27b-4d4a-8371-6cf837dea4d2@ hk1.helloco.xyz:60001?mode=multi&
    // security=reality&encryption=none&type=tcp& flow=xtls-rprx-vision&
    // pbk=Kyrdn7OhtL66JwSRScElBxoFSZLr5beafP4njt_Y_G0&sid=a3ffb25d& sni=python.org&
    // servername=python.org&spx=%2F&fp=ios#Hong+Kong+01
    // vless://b3524347-d27b-4d4a-8371-6cf837dea4d2@ hk2.helloco.xyz:60001?mode=multi&
    // security=reality&encryption=none&type=tcp& flow=xtls-rprx-vision&
    // pbk=Kyrdn7OhtL66JwSRScElBxoFSZLr5beafP4njt_Y_G0&sid=a3ffb25d& sni=python.org&
    // servername=python.org&spx=%2F&fp=qq#Hong+Kong+02
    // vless://b3524347-d27b-4d4a-8371-6cf837dea4d2@ id1.helloco.xyz:60001?mode=multi&
    // security=reality&encryption=none&type=tcp& flow=xtls-rprx-vision&
    // pbk=Kyrdn7OhtL66JwSRScElBxoFSZLr5beafP4njt_Y_G0&sid=a3ffb25d& sni=python.org&
    // servername=python.org&spx=%2F&fp=ios#Indonesia+01
    // vless://b3524347-d27b-4d4a-8371-6cf837dea4d2@ id2.helloco.xyz:60001?mode=multi&
    // security=reality&encryption=none&type=tcp& flow=xtls-rprx-vision&
    // pbk=Kyrdn7OhtL66JwSRScElBxoFSZLr5beafP4njt_Y_G0&sid=a3ffb25d& sni=python.org&
    // servername=python.org&spx=%2F&fp=safari#Indonesia+02
    // vless://b3524347-d27b-4d4a-8371-6cf837dea4d2@ sg1.helloco.xyz:60001?mode=multi&
    // security=reality&encryption=none&type=tcp& flow=xtls-rprx-vision&
    // pbk=Kyrdn7OhtL66JwSRScElBxoFSZLr5beafP4njt_Y_G0&sid=a3ffb25d& sni=python.org&
    // servername=python.org&spx=%2F&fp=safari#Singapore+01
    // vless://b3524347-d27b-4d4a-8371-6cf837dea4d2@ sg2.helloco.xyz:60001?mode=multi&
    // security=reality&encryption=none&type=tcp& flow=xtls-rprx-vision&
    // pbk=Kyrdn7OhtL66JwSRScElBxoFSZLr5beafP4njt_Y_G0&sid=a3ffb25d& sni=python.org&
    // servername=python.org&spx=%2F&fp=safari#Singapore+02
    // vless://b3524347-d27b-4d4a-8371-6cf837dea4d2@ sg3.helloco.xyz:60001?mode=multi&
    // security=reality&encryption=none&type=tcp& flow=xtls-rprx-vision&
    // pbk=Kyrdn7OhtL66JwSRScElBxoFSZLr5beafP4njt_Y_G0&sid=a3ffb25d& sni=python.org&
    // servername=python.org&spx=%2F&fp=qq#Singapore+03
    // vless://b3524347-d27b-4d4a-8371-6cf837dea4d2@ sg4.helloco.xyz:60001?mode=multi&
    // security=reality&encryption=none&type=tcp& flow=xtls-rprx-vision&
    // pbk=Kyrdn7OhtL66JwSRScElBxoFSZLr5beafP4njt_Y_G0&sid=a3ffb25d& sni=python.org&
    // servername=python.org&spx=%2F&fp=qq#Singapore+04
    // vless://b3524347-d27b-4d4a-8371-6cf837dea4d2@ uk1.helloco.xyz:60001?mode=multi&
    // security=reality&encryption=none&type=tcp& flow=xtls-rprx-vision&
    // pbk=Kyrdn7OhtL66JwSRScElBxoFSZLr5beafP4njt_Y_G0&sid=a3ffb25d& sni=python.org&
    // servername=python.org&spx=%2F&fp=firefox#United+Kindom+01
    // vless://b3524347-d27b-4d4a-8371-6cf837dea4d2@ uk2.helloco.xyz:60001?mode=multi&
    // security=reality&encryption=none&type=tcp& flow=xtls-rprx-vision&
    // pbk=Kyrdn7OhtL66JwSRScElBxoFSZLr5beafP4njt_Y_G0&sid=a3ffb25d& sni=python.org&
    // servername=python.org&spx=%2F&fp=ios#United+Kindom+02
}
