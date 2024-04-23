use std::net::IpAddr;
use std::time::Duration;
use reqwest::{Client, Error};
use serde::Deserialize;

#[allow(dead_code)]
pub async fn get_ip_detail(ip_addr: &IpAddr) -> Result<IpSbDetail, Error> {
    let client = Client::builder().timeout(Duration::from_secs(30)).build()?;
    let url = format!("https://api.ip.sb/geoip/{}", ip_addr);
    let res = client.get(url).send().await?;
    let result = res.json::<IpSbDetail>().await?;
    Ok(result)
}

pub async fn get_ip_detail_with_proxy(ip_addr: &IpAddr, proxy_url: &str) -> Result<IpSbDetail, Error> {
    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .proxy(reqwest::Proxy::all(proxy_url)?)
        .build()?;
    let url = format!("https://api.ip.sb/geoip/{}", ip_addr);
    let res = client.get(url).send().await?;
    let result = res.json::<IpSbDetail>().await?;
    Ok(result)
}

#[derive(Debug, Deserialize)]
pub struct IpSbDetail {
    pub ip: String,
    pub country: String,
    pub country_code: String,
    pub isp: String,
    pub organization: String,
    pub city: String,
    pub region: String,
    pub region_code: String,
    pub asn: u32,
    pub asn_organization: String,
    pub longitude: f64,
    pub latitude: f64,
    pub offset: i32,
    pub postal_code: String,
    pub continent_code: String,
    pub timezone: String,
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;
    use super::*;

    #[tokio::test]
    async fn test_ip_detail() {
        println!("{:?}", get_ip_detail(&IpAddr::from_str("2001:470:f2da:2d87:add2:f8cd:611d:2364").unwrap()).await.unwrap());
        println!("{:?}", get_ip_detail(&IpAddr::from_str("209.141.57.3").unwrap()).await.unwrap());
    }
}