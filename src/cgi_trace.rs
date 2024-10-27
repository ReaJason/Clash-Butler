use futures_util::future::{select_ok, BoxFuture};
use futures_util::FutureExt;
use reqwest::Client;
use serde_json::Value;
use std::collections::HashMap;
use std::net::IpAddr;
use std::str::FromStr;
use std::time::Duration;
use tokio::time::sleep;
use tracing::log::error;

const OPENAI_TRACE_URL: &str = "https://chat.openai.com/cdn-cgi/trace";
const CF_TRACE_URL: &str = "https://1.0.0.1/cdn-cgi/trace";

#[allow(unused)]
const CF_CN_TRACE_URL: &str = "https://cf-ns.com/cdn-cgi/trace";

// IP 查询超时时间
const TIMEOUT: Duration = Duration::from_millis(1000);

type IpBoxFuture<'a> = BoxFuture<'a, Result<(IpAddr, &'a str), Box<dyn std::error::Error>>>;

pub async fn get_ip(proxy_url: &str) -> Result<(IpAddr, &str), Box<dyn std::error::Error>> {
    let cf_future: IpBoxFuture = async {
        match get_trace_info_with_proxy(proxy_url, CF_TRACE_URL).await {
            Ok(trace) => Ok((trace.ip, "cf")),
            Err(e) => {
                error!("从 Cloudflare 获取 IP 失败, {e}");
                Err(e)
            }
        }
    }
    .boxed();

    let ipify_future: IpBoxFuture = async {
        match get_ip_by_ipify(proxy_url).await {
            Ok(ip) => Ok((ip, "ipify")),
            Err(e) => {
                error!("从 ipify 获取 IP 失败, {e}");
                Err(e)
            }
        }
    }
    .boxed();

    let openai_future: IpBoxFuture = async {
        match get_trace_info_with_proxy(proxy_url, OPENAI_TRACE_URL).await {
            Ok(trace) => Ok((trace.ip, "openai")),
            Err(e) => {
                error!("从 OpenAI 获取 IP 失败, {e}");
                Err(e)
            }
        }
    }
    .boxed();

    let futures = vec![cf_future, ipify_future, openai_future];
    match select_ok(futures).await {
        Ok(((ip, from), _)) => Ok((ip, from)),
        Err(_) => Err("获取不到 IP 地址，可能节点已失效，已过滤".into()),
    }
}

// clash 规则走的是国内，没走代理所以寄
#[allow(dead_code)]
async fn get_ip_by_ipip(proxy_url: &str) -> Result<IpAddr, Box<dyn std::error::Error>> {
    let client = Client::builder()
        .timeout(TIMEOUT)
        .proxy(reqwest::Proxy::all(proxy_url)?)
        .build()?;

    let response = client.get("https://myip.ipip.net/ip").send().await?;
    let body = response.text().await?;

    let value: Value = serde_json::from_str(&body)?;

    if let Some(ip_str) = value.get("ip").and_then(|v| v.as_str()) {
        if let Ok(ip) = IpAddr::from_str(ip_str) {
            return Ok(ip);
        }
    }
    Err("Failed to parse IP address".into())
}

async fn get_ip_by_ipify(proxy_url: &str) -> Result<IpAddr, Box<dyn std::error::Error>> {
    let client = Client::builder()
        .timeout(TIMEOUT)
        .proxy(reqwest::Proxy::all(proxy_url)?)
        .build()?;

    let response = client
        .get("https://api4.ipify.org/?format=json")
        .send()
        .await?;
    let body = response.text().await?;

    let value: Value = serde_json::from_str(&body)?;

    if let Some(ip_str) = value.get("ip").and_then(|v| v.as_str()) {
        if let Ok(ip) = IpAddr::from_str(ip_str) {
            return Ok(ip);
        }
    }
    Err("Failed to parse IP address".into())
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

async fn get_trace_info_with_proxy(
    proxy_url: &str,
    trace_url: &str,
) -> Result<TraceInfo, Box<dyn std::error::Error>> {
    let client = Client::builder()
        .timeout(TIMEOUT)
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
                    return Err(e.into());
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

    const PROXY_URL: &str = "http://127.0.0.1:7999";

    #[tokio::test]
    #[ignore]
    async fn test_get_ip() {
        let result = get_ip(PROXY_URL).await;
        println!("{:?}", result.unwrap())
    }

    #[tokio::test]
    #[ignore]
    async fn test_get_trace_info_with_proxy() {
        let result = get_trace_info_with_proxy(PROXY_URL, OPENAI_TRACE_URL).await;
        println!("{:?}", result);
        let result = get_trace_info_with_proxy(PROXY_URL, CF_TRACE_URL).await;
        println!("{:?}", result);
        let result = get_trace_info_with_proxy(PROXY_URL, CF_CN_TRACE_URL).await;
        println!("{:?}", result);
    }

    #[tokio::test]
    #[ignore]
    async fn test_get_ip_from_ipip() {
        let result = get_ip_by_ipip(PROXY_URL).await;
        println!("{:?}", result);
        let result = get_ip_by_ipify(PROXY_URL).await;
        println!("{:?}", result);
    }
}
