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
    let client = reqwest::Client::builder().redirect(reqwest::redirect::Policy::limited(10)).build()?;

    // Get latest release
    let url = format!("https://api.github.com/repos/{repo}/releases/latest");
    let resp = client
        .get(&url)
        .header("Authorization", format!("token {token}"))
        .header("User-Agent", "pulse-cli")
        .header("Accept", "application/vnd.github+json")
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        bail!("GitHub API falhou ({status}): {}", &body[..body.len().min(200)]);
    }

    let release: Release = resp.json().await?;

    // Find the asset
    let asset = release.assets.iter().find(|a| a.name == binary_name).ok_or_else(|| {
        let available: Vec<&str> = release.assets.iter().map(|a| a.name.as_str()).collect();
        anyhow::anyhow!(
            "Asset '{binary_name}' não encontrado na release {}. Disponíveis: {:?}",
            release.tag_name,
            available
        )
    })?;

    // Download asset binary (GitHub redirects to S3)
    let download_url = format!("https://api.github.com/repos/{repo}/releases/assets/{}", asset.id);
    let resp = client
        .get(&download_url)
        .header("Authorization", format!("token {token}"))
        .header("User-Agent", "pulse-cli")
        .header("Accept", "application/octet-stream")
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        bail!("Download falhou ({status}). Verifique se o token tem scope 'repo'.");
    }

    let bytes = resp.bytes().await?;
    if bytes.is_empty() {
        bail!("Download retornou vazio para {binary_name}");
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
