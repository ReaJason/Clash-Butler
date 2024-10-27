#![allow(dead_code)]

use scraper::Html;
use scraper::Selector;
use tracing::log;

pub async fn is_clean_proxy(proxy_port: i64) -> (String, bool) {
    is_clean(Some(proxy_port)).await
}

async fn is_clean_ip() -> (String, bool) {
    is_clean(None).await
}

async fn is_clean(proxy_port: Option<i64>) -> (String, bool) {
    let client;
    if let Some(port) = proxy_port {
        let proxy = reqwest::Proxy::all(format!("http://127.0.0.1:{}", port)).unwrap();
        client = reqwest::Client::builder().proxy(proxy).build().unwrap();
    } else {
        client = reqwest::Client::new();
    }

    let request = client.post("https://whatismyiplookup.com/index.php");

    let response = request.send().await.unwrap();
    let body = response.text().await.unwrap();

    let html = Html::parse_document(&body);
    log::debug!("{:?}", html);
    let ip_selector = Selector::parse(r#"#lisfw > div > div:nth-child(2) > h3 > span"#).unwrap();
    let vpn_selector =
        Selector::parse(r#"#lisfw > div > div:nth-last-child(2) > h3 > span"#).unwrap();
    let ip = html.select(&ip_selector).next().unwrap().inner_html();
    let is_clean = html
        .select(&vpn_selector)
        .next()
        .unwrap()
        .inner_html()
        .contains("Clean IP");

    if is_clean {
        log::info!("{ip} is a clean ip");
    } else {
        log::info!("{ip} is not a clean ip")
    }
    (ip, is_clean)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_without_proxy() {
        let ip_info = is_clean_ip().await;
        println!("{:?}", ip_info);
        assert!(!ip_info.0.is_empty())
    }

    #[tokio::test]
    #[ignore]
    async fn test_get_risk() {
        let ip_info = is_clean_proxy(7890).await;
        println!("{:?}", ip_info);
        assert!(!ip_info.0.is_empty())
    }
}
