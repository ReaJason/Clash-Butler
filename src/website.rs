use std::time::Duration;

use anyhow::anyhow;
use anyhow::Context;
use anyhow::Result;
use reqwest::Client;
use reqwest::StatusCode;

const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/102.0.5005.63 Safari/537.36";
const TIMEOUT: Duration = Duration::from_secs(5);

fn build_client(proxy_url: &str) -> Result<Client> {
    Client::builder()
        .proxy(reqwest::Proxy::all(proxy_url).context("Failed to create proxy configuration")?)
        .timeout(TIMEOUT)
        .build()
        .context("Failed to build HTTP client")
}

pub async fn claude_is_ok(proxy_url: &str) -> Result<()> {
    let url = "https://claude.ai/login";
    let client = build_client(proxy_url)?;
    let resp = client
        .get(url)
        .header("User-Agent", USER_AGENT)
        .send()
        .await?;
    let status = resp.status();
    let text = resp.text().await?;
    if text.contains("unavailable") {
        return Err(anyhow!("app unavailable"));
    }
    if status.is_success() {
        return Ok(());
    }
    Err(anyhow!("http status code: {}", status))
}

pub async fn openai_is_ok(proxy_url: &str) -> Result<()> {
    let url = "https://auth.openai.com/favicon.ico";
    let client = build_client(proxy_url)?;
    let resp = client
        .get(url)
        .header("User-Agent", USER_AGENT)
        .send()
        .await
        .with_context(|| "Failed to send request to OpenAI")?;
    let status = resp.status();
    if status == StatusCode::OK {
        return Ok(());
    }
    Err(anyhow!("error status code: {}", status))
}

#[allow(dead_code)]
pub async fn youtube_music_is_ok(proxy_url: &str) -> Result<bool> {
    let url = "https://music.youtube.com/generate_204";
    let client = build_client(proxy_url)?;
    let resp = client
        .head(url)
        .header("User-Agent", USER_AGENT)
        .send()
        .await
        .with_context(|| "Failed to send request to OpenAI")?;
    let status = resp.status();
    if status == StatusCode::NO_CONTENT {
        return Ok(true);
    }
    Err(anyhow!("error status code: {}", status))
}

mod test {
    #[tokio::test]
    async fn test_claude_is_ok() {
        let result = super::claude_is_ok("http://localhost:7890").await;
        assert!(result.is_ok())
    }
    #[tokio::test]
    async fn test_openai_is_ok() {
        let result = super::openai_is_ok("http://localhost:7890").await;
        println!("{:#?}", result);
    }
}
