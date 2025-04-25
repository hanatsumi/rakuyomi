use std::time::Duration;

use anyhow::Context;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct CheckUpdateResponse {
    pub available: bool,
    pub current_version: String,
    pub latest_version: String,
    pub release_url: String,
}

#[derive(Deserialize)]
struct GitHubRelease {
    tag_name: String,
    html_url: String,
}

pub async fn check_update(current_version: String) -> anyhow::Result<CheckUpdateResponse> {
    // Get latest release from GitHub API
    let client = reqwest::Client::new();
    let response = client
        .get("https://api.github.com/repos/hanatsumi/rakuyomi/releases/latest")
        .header("User-Agent", "rakuyomi")
        .timeout(Duration::from_secs(10))
        .send()
        .await
        .context("Failed to fetch latest release")?
        .error_for_status()
        .context("Failed to fetch latest release")?;

    let latest_release = response
        .json::<GitHubRelease>()
        .await
        .context("Failed to parse GitHub API response")?;

    let latest_version = latest_release.tag_name.trim_start_matches('v').to_string();
    let available = latest_version != current_version && !latest_version.is_empty();

    Ok(CheckUpdateResponse {
        available,
        current_version,
        latest_version,
        release_url: latest_release.html_url,
    })
}
