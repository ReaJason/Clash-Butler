use std::time::Duration;

use anyhow::anyhow;
use anyhow::Context;
use anyhow::Result;
use regex::Regex;
use reqwest::redirect::Policy;
use reqwest::Client;
use reqwest::StatusCode;

const USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/138.0.0.0 Safari/537.36";

fn build_client(proxy_url: &str, timeout: Duration) -> Result<Client> {
    Client::builder()
        .proxy(reqwest::Proxy::all(proxy_url).context("Failed to create proxy configuration")?)
        .redirect(Policy::none())
        .timeout(timeout)
        .build()
        .context("Failed to build HTTP client")
}

#[allow(dead_code)]
pub async fn claude_is_ok(proxy_url: &str, timeout: Duration) -> Result<()> {
    let url = "https://claude.ai/favicon.ico";
    let client = build_client(proxy_url, timeout)?;
    let resp = client
        .get(url)
        .header("User-Agent", USER_AGENT)
        .send()
        .await?;
    let status = resp.status();
    if status.is_redirection() {
        let location = resp
            .headers()
            .get("Location")
            .ok_or_else(|| anyhow!("redirect but no location"))?;
        if location.to_str()?.contains("unavailable") {
            return Err(anyhow!("app unavailable"));
        }
        return Err(anyhow!("redirect location: {} ", location.to_str()?));
    }
    if status.is_success() {
        return Ok(());
    }
    let text = resp.text().await?;
    if text.contains("unavailable") {
        return Err(anyhow!("app unavailable"));
    }
    if text.contains("Just a moment") {
        // CF 盾，节点质量不行
        return Err(anyhow!("banned by cloudflare"));
    }
    Err(anyhow!("http status code: {}", status))
}

#[allow(dead_code)]
pub async fn openai_is_ok(proxy_url: &str, timeout: Duration) -> Result<()> {
    let url = "https://auth.openai.com/favicon.ico";
    let client = build_client(proxy_url, timeout)?;
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
pub async fn youtube_music_is_ok(proxy_url: &str, timeout: Duration) -> Result<()> {
    let url = "https://music.youtube.com/generate_204";
    let client = build_client(proxy_url, timeout)?;
    let resp = client
        .head(url)
        .header("User-Agent", USER_AGENT)
        .send()
        .await
        .with_context(|| "Failed to send request to YoutubeMusic")?;
    let status = resp.status();
    if status == StatusCode::NO_CONTENT {
        return Ok(());
    }
    Err(anyhow!("error status code: {}", status))
}

pub async fn gemini_is_ok(proxy_url: &str, timeout: Duration) -> Result<()> {
    let url = "https://gemini.google.com";
    let client = build_client(proxy_url, timeout)?;
    let resp = client
        .get(url)
        .header("User-Agent", USER_AGENT)
        .send()
        .await
        .with_context(|| "Failed to send request to Gemini")?;
    let status = resp.status();
    if 429 == status {
        // 万人骑的代理 IP
        return Err(anyhow!("banned for unusual traffic"));
    }
    if status.is_redirection() {
        let location = resp
            .headers()
            .get("Location")
            .ok_or_else(|| anyhow!("redirect but no location"))?;
        let location_str = location.to_str()?;
        if location_str.starts_with("https://www.google.com/sorry/index")
            || location_str.starts_with("https://consent.google.com/m")
        {
            return Err(anyhow!("banned for unusual traffic"));
        }
        return Err(anyhow!("redirect location: {} ", location_str));
    }
    let text = resp.text().await?;
    if text.contains("45631641,null,true") {
        return Ok(());
    }
    let re = Regex::new(r#"2,\s*1,\s*200,\s*"([A-Z]+)""#)?;
    if let Some(caps) = re.captures(text.as_str()) {
        let region = caps[1].to_string();
        return Err(anyhow!("unsupported region: {}", region));
    }
    Err(anyhow!("error status code: {}", status))
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    #[ignore]
    async fn test_gemini_is_ok() {
        let result = gemini_is_ok("http://localhost:7890", Duration::from_secs(5)).await;
        println!("{:?}", result);
    }

    #[tokio::test]
    #[ignore]
    async fn test_youtube_music_is_ok() {
        let result = youtube_music_is_ok("http://localhost:7890", Duration::from_secs(5)).await;
        println!("{:?}", result);
    }

    #[tokio::test]
    #[ignore]
    async fn test_claude_is_ok() {
        let result = claude_is_ok("http://localhost:7890", Duration::from_secs(5)).await;
        println!("{:?}", result);
    }

    #[tokio::test]
    #[ignore]
    async fn test_openai_is_ok() {
        let result = openai_is_ok("http://localhost:7890", Duration::from_secs(5)).await;
        println!("{:?}", result);
    }
}
