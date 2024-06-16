use reqwest;
use std::time::{Instant, Duration};
use futures_util::StreamExt;
use reqwest::Proxy;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[allow(unused)]
pub struct SpeedTestConfig {
    pub enabled: bool,
    pub url: String,
    pub timeout: u16,
}

#[allow(dead_code)]
async fn test_download(url: &str, timeout: Duration, proxy_url: Option<&str>) -> Result<(Duration, f64, Duration), reqwest::Error> {
    let client_builder = reqwest::Client::builder()
        .timeout(timeout);

    let client = if let Some(proxy) = proxy_url {
        client_builder.proxy(Proxy::all(proxy)?).build()?
    } else {
        client_builder.build()?
    };

    let start = Instant::now();
    let response = client.get(url).send().await?;

    // Stream the response body
    let mut stream = response.bytes_stream();
    let mut total_bytes = 0;
    let first_byte_time = if let Some(chunk) = stream.next().await {
        total_bytes += chunk?.len();
        start.elapsed() // TTFB is the elapsed time when the first byte is received
    } else {
        Duration::from_secs(0) // No bytes received
    };

    while let Some(chunk) = stream.next().await {
        total_bytes += chunk?.len();
    }
    let total_duration = start.elapsed();
    let bandwidth = (total_bytes as f64 / 1024.0) / total_duration.as_secs_f64();  // KB per second
    Ok((total_duration, bandwidth, first_byte_time))
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn test_download() {
        let url = "https://speed.cloudflare.com/__down?bytes=1024";  // 100MB download
        match crate::speedtest::test_download(url, Duration::from_secs(10), Some("http://127.0.0.1:7890")).await {
            Ok(result) => println!("{:?}", result),
            Err(e) => eprintln!("{:?}", e)
        }
    }
}

// #[tokio::main]
// async fn main() {
//     let url = "https://speed.cloudflare.com/__down?bytes=104857600";  // 100MB download
//     let proxy_url = "http://127.0.0.1:7890";
//     let result = test_download(url, proxy_url).await.unwrap();
//     println!("{:?}", result);
//
//
//     // let handles = (0..5).map(|_| {
//     //     tokio::spawn(async move {
//     //         test_download(url, proxy_url).await.unwrap()
//     //     })
//     // });
//     //
//     // let mut total_ttfb: Duration = Duration::new(0, 0);
//     // let mut total_bandwidth = 0.0;
//     // let mut results = Vec::new();
//     // for handle in handles {
//     //     let result = handle.await.unwrap();
//     //     results.push(result);
//     //     total_ttfb += result.2;
//     //     total_bandwidth += result.1;
//     // }
//     //
//     // let avg_ttfb = total_ttfb / results.len() as u32;
//     // let avg_bandwidth = total_bandwidth / results.len() as f64;
//
//     // println!("Average TTFB: {:.2?} seconds, Average Bandwidth: {:.2} bytes/sec", avg_ttfb, avg_bandwidth);
// }