use anyhow::{Result, bail};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Release {
    tag_name: String,
    assets: Vec<Asset>,
}

#[derive(Debug, Deserialize)]
struct Asset {
    name: String,
    id: u64,
    size: u64,
}

/// Download a release binary from a GitHub private repo.
/// Returns (version_tag, binary_bytes).
pub async fn download_release_binary(token: &str, repo: &str, binary_name: &str) -> Result<(String, Vec<u8>)> {
    let client = reqwest::Client::new();

    // Get latest release
    let release: Release = client
        .get(format!("https://api.github.com/repos/{repo}/releases/latest"))
        .header("Authorization", format!("token {token}"))
        .header("User-Agent", "pulse-cli")
        .header("Accept", "application/vnd.github+json")
        .send()
        .await?
        .json()
        .await?;

    // Find the asset
    let asset = release
        .assets
        .iter()
        .find(|a| a.name == binary_name)
        .ok_or_else(|| anyhow::anyhow!("Asset '{binary_name}' not found in release {}", release.tag_name))?;

    // Download asset (requires Accept: application/octet-stream)
    let bytes = client
        .get(format!("https://api.github.com/repos/{repo}/releases/assets/{}", asset.id))
        .header("Authorization", format!("token {token}"))
        .header("User-Agent", "pulse-cli")
        .header("Accept", "application/octet-stream")
        .send()
        .await?
        .bytes()
        .await?;

    if bytes.is_empty() {
        bail!("Downloaded empty binary for {binary_name}");
    }

    Ok((release.tag_name, bytes.to_vec()))
}

/// Determine the correct binary asset name for a given component + arch.
pub fn asset_name(component: &str, arch: &str) -> String {
    match arch {
        "x86_64" => format!("pulse-{component}-linux-x86_64"),
        "aarch64" => format!("pulse-{component}-linux-aarch64"),
        _ => format!("pulse-{component}-linux-{arch}"),
    }
}
