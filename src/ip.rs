use std::net::IpAddr;
use std::time::Duration;
use reqwest::{Client, Error};
use serde::{Deserialize, Serialize};
use tracing::log::error;

// IP 详情查询超时时间
const TIMEOUT: Duration = Duration::from_millis(1000);

pub async fn get_ip_detail(ip_addr: &IpAddr, proxy_url: &str) -> Result<IpDetail, Box<dyn std::error::Error>> {
    match get_ip_detail_from_ipsb(ip_addr, proxy_url).await {
        Ok(ip_detail) => return Ok(ip_detail),
        Err(err) => error!("从 ipSb 获取 IP 详情失败, {err}")
    }

    match get_ip_detail_from_ipapi(ip_addr, proxy_url).await {
        Ok(ip_detail) => return Ok(ip_detail),
        Err(err) => error!("从 ipApi 获取 IP 详情失败, {err}")
    }

    Err("获取 IP 详情失败".into())
}

pub async fn get_ip_detail_from_ipsb(ip_addr: &IpAddr, proxy_url: &str) -> Result<IpDetail, Error> {
    let client = Client::builder()
        .timeout(TIMEOUT)
        .proxy(reqwest::Proxy::all(proxy_url)?)
        .build()?;
    let url = format!("https://api.ip.sb/geoip/{}", ip_addr);
    let res = client.get(url).send().await?;
    let result = res.json::<IpDetail>().await?;
    Ok(result)
}

#[allow(dead_code)]
pub async fn get_ip_detail_from_ipapi(ip_addr: &IpAddr, proxy_url: &str) -> Result<IpDetail, Error> {
    let client = Client::builder()
        .timeout(TIMEOUT)
        .proxy(reqwest::Proxy::all(proxy_url)?)
        .build()?;
    let url = format!("http://ip-api.com/json/{}", ip_addr);
    let res = client.get(url).send().await?;
    let ip_api_detail = res.json::<IpApiDetail>().await?;
    Ok(IpDetail {
        ip: ip_api_detail.query,
        country: ip_api_detail.country,
        country_code: ip_api_detail.country_code,
        isp: ip_api_detail.isp,
        city: ip_api_detail.city,
        region: ip_api_detail.region_name,
        region_code: ip_api_detail.region,
        timezone: ip_api_detail.timezone,
    })
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IpDetail {
    pub ip: String,
    pub country: String,
    pub country_code: String,
    pub isp: String,
    pub city: String,
    pub region: String,
    pub region_code: String,
    pub timezone: String,
}

#[derive(Debug, Deserialize)]
pub struct IpApiDetail {
    pub query: String,
    pub country: String,
    #[serde(rename = "countryCode")]
    pub country_code: String,
    pub isp: String,
    pub city: String,
    pub region: String,
    #[serde(rename = "regionName")]
    pub region_name: String,
    pub timezone: String,
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;
    use super::*;

    const PROXY_URL: &str = "http://127.0.0.1:7890";

    #[tokio::test]
    async fn test_ip_detail() {
        let result = get_ip_detail(&IpAddr::from_str("152.67.206.156").unwrap(), PROXY_URL).await;
        println!("{:?}", result);
    }
}