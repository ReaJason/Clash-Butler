use std::collections::HashMap;
use std::net::IpAddr;
use std::str::FromStr;
use std::time::Duration;

use reqwest::{Client, Error};
use tokio::time::sleep;

const OPENAI_TRACE_URL: &str = "https://chat.openai.com/cdn-cgi/trace";
const CF_TRACE_URL: &str = "https://1.0.0.1/cdn-cgi/trace";

#[allow(dead_code)]
pub async fn get_ip_by_openai(proxy_url: &str) -> Result<IpAddr, Error> {
    Ok(get_trace_info_with_proxy(proxy_url, OPENAI_TRACE_URL).await?.ip)
}

pub async fn get_ip_by_cloudflare(proxy_url: &str) -> Result<IpAddr, Error> {
    Ok(get_trace_info_with_proxy(proxy_url, CF_TRACE_URL).await?.ip)
}

fn parse_trace_info(text: String) -> TraceInfo {
    let mut map = HashMap::new();
    for line in text.lines() {
        let parts: Vec<&str> = line.split('=').collect();
        if parts.len() == 2 {
            map.insert(parts[0].to_string(), parts[1].to_string());
        }
    }

    TraceInfo {
        fl: map.get("fl").unwrap_or(&String::new()).clone(),
        h: map.get("h").unwrap_or(&String::new()).clone(),
        ip: IpAddr::from_str(&map.get("ip").unwrap().clone()).unwrap(),
        ts: map.get("ts").unwrap_or(&String::new()).clone(),
        visit_scheme: map.get("visit_scheme").unwrap_or(&String::new()).clone(),
        uag: map.get("uag").unwrap_or(&String::new()).clone(),
        colo: map.get("colo").unwrap_or(&String::new()).clone(),
        sliver: map.get("sliver").unwrap_or(&String::new()).clone(),
        http: map.get("http").unwrap_or(&String::new()).clone(),
        loc: map.get("loc").unwrap_or(&String::new()).clone(),
        tls: map.get("tls").unwrap_or(&String::new()).clone(),
        sni: map.get("sni").unwrap_or(&String::new()).clone(),
        warp: map.get("warp").unwrap_or(&String::new()).clone(),
        gateway: map.get("gateway").unwrap_or(&String::new()).clone(),
        rbi: map.get("rbi").unwrap_or(&String::new()).clone(),
        kex: map.get("kex").unwrap_or(&String::new()).clone(),
    }
}

#[allow(dead_code)]
async fn get_trace_info(trace_url: &str) -> Result<TraceInfo, Error> {
    let client = Client::builder()
        .timeout(Duration::from_secs(1))
        .build()?;
    let res = client.get(trace_url).send().await?;
    let body = res.text().await?;
    Ok(parse_trace_info(body))
}

async fn get_trace_info_with_proxy(proxy_url: &str, trace_url: &str) -> Result<TraceInfo, Error> {
    let client = Client::builder()
        .timeout(Duration::from_secs(1))
        .proxy(reqwest::Proxy::all(proxy_url)?)
        .build()?;

    let mut attempts = 0;
    let max_attempts = 3;

    while attempts < max_attempts {
        match client.get(trace_url).send().await {
            Ok(res) => {
                let body = res.text().await?;
                return Ok(parse_trace_info(body));
            }
            Err(e) => {
                if attempts + 1 == max_attempts {
                    return Err(e);
                }
                attempts += 1;
                sleep(Duration::from_secs(1)).await; // 等待一秒再重试
            }
        }
    }
    unreachable!()
}


#[derive(Debug)]
#[allow(unused)]
struct TraceInfo {
    fl: String,
    h: String,
    ip: IpAddr,
    ts: String,
    visit_scheme: String,
    uag: String,
    colo: String,
    sliver: String,
    http: String,
    loc: String,
    tls: String,
    sni: String,
    warp: String,
    gateway: String,
    rbi: String,
    kex: String,
}


#[cfg(test)]
mod tests {
    use super::*;

    const PROXY_URL: &str = "http://127.0.0.1:7890";

    #[tokio::test]
    #[ignore]
    async fn test_get_trace_info() {
        let result = get_trace_info(CF_TRACE_URL).await;
        assert!(result.is_ok());
        let output = result.unwrap();
        println!("{:?}", output);
    }

    #[tokio::test]
    #[ignore]
    async fn test_get_trace_info_with_proxy() {
        let result = get_trace_info_with_proxy(PROXY_URL, OPENAI_TRACE_URL).await;
        println!("{:?}", result);
        let result = get_trace_info_with_proxy(PROXY_URL, CF_TRACE_URL).await;
        println!("{:?}", result);
    }
}