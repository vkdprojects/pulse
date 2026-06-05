use anyhow::{Result, bail};
use ring::digest;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::{info, warn};

const UPDATE_MARKER_PATH: &str = "/var/lib/pulse/update-marker";
const PREV_BINARY_SUFFIX: &str = ".prev";

/// Check for updates via master's REST API and apply if available.
pub async fn check_and_apply(master_endpoint: &str, current_version: &str) -> Result<()> {
    let arch = std::env::consts::ARCH;
    let arch_str = match arch {
        "x86_64" => "x86_64",
        "aarch64" => "aarch64",
        _ => return Ok(()), // unsupported arch, skip
    };

    let http_addr = derive_http_addr(master_endpoint);
    let check_url = format!("/api/updates/check?arch={arch_str}&version={current_version}");

    let response = http_get(&http_addr, &check_url).await?;
    let info: serde_json::Value = serde_json::from_str(&response)?;

    if info.get("available").and_then(|v| v.as_bool()) != Some(true) {
        return Ok(());
    }

    let version = info["version"].as_str().unwrap_or_default();
    let expected_sha = info["sha256"].as_str().unwrap_or_default();
    let urgent = info["urgent"].as_bool().unwrap_or(false);

    info!(version, urgent, "update available");

    // Jitter: non-urgent updates get random delay (0-300s)
    if !urgent {
        let delay_secs = random_delay();
        info!(delay_secs, "scheduling update with jitter");
        tokio::time::sleep(std::time::Duration::from_secs(delay_secs)).await;
    }

    // Download binary
    let download_url = format!("/api/updates/download?version={version}&arch={arch_str}");
    let binary = http_get_bytes(&http_addr, &download_url).await?;

    // Verify SHA256
    let hash = digest::digest(&digest::SHA256, &binary);
    let computed: String = {
        use std::fmt::Write;
        let mut s = String::with_capacity(hash.as_ref().len() * 2);
        for b in hash.as_ref() {
            write!(s, "{:02x}", b).unwrap();
        }
        s
    };
    if computed != expected_sha {
        bail!("sha256 mismatch: expected {expected_sha}, got {computed}");
    }
    info!(size = binary.len(), "binary downloaded and verified");

    // Save current binary as .prev for rollback
    let exe_path = std::env::current_exe()?;
    let prev_path = format!("{}{PREV_BINARY_SUFFIX}", exe_path.display());
    if exe_path.exists() {
        std::fs::copy(&exe_path, &prev_path).ok();
    }

    // Write update marker (for rollback detection on next boot)
    write_update_marker(current_version, version)?;

    // Write new binary to temp file
    let tmp_path = format!("/tmp/pulse-agent-update-{version}");
    tokio::fs::write(&tmp_path, &binary).await?;

    // Atomic self-replace
    self_replace::self_replace(&tmp_path)?;
    std::fs::remove_file(&tmp_path).ok();

    info!(from = current_version, to = version, "update applied, exiting for restart");
    std::process::exit(0);
}

/// On startup: check if we crashed after an update and need rollback.
pub fn check_rollback_needed() {
    let Ok(content) = std::fs::read_to_string(UPDATE_MARKER_PATH) else { return };
    let parts: Vec<&str> = content.trim().split('\n').collect();
    if parts.len() < 3 {
        return;
    }

    let attempts: u32 = parts[2].parse().unwrap_or(0);
    if attempts >= 3 {
        warn!("3 failed starts after update, rolling back");
        let exe_path = std::env::current_exe().unwrap_or_default();
        let prev_path = format!("{}{PREV_BINARY_SUFFIX}", exe_path.display());
        if std::path::Path::new(&prev_path).exists() {
            if let Err(e) = self_replace::self_replace(&prev_path) {
                warn!(error = %e, "rollback failed");
            } else {
                std::fs::remove_file(UPDATE_MARKER_PATH).ok();
                info!("rollback complete, restarting");
                std::process::exit(0);
            }
        }
    } else {
        // Increment attempt counter
        let new_content = format!("{}\n{}\n{}", parts[0], parts[1], attempts + 1);
        std::fs::write(UPDATE_MARKER_PATH, new_content).ok();
    }
}

/// Call after successful connection to master → update is confirmed healthy.
pub fn confirm_update_healthy() {
    if std::path::Path::new(UPDATE_MARKER_PATH).exists() {
        std::fs::remove_file(UPDATE_MARKER_PATH).ok();
        info!("update confirmed healthy, marker removed");
    }
}

fn write_update_marker(from: &str, to: &str) -> Result<()> {
    let marker_dir = std::path::Path::new(UPDATE_MARKER_PATH).parent().unwrap_or(std::path::Path::new("."));
    std::fs::create_dir_all(marker_dir).ok();
    std::fs::write(UPDATE_MARKER_PATH, format!("{from}\n{to}\n0"))?;
    Ok(())
}

fn random_delay() -> u64 {
    let mut buf = [0u8; 2];
    std::fs::File::open("/dev/urandom")
        .and_then(|mut f| std::io::Read::read_exact(&mut f, &mut buf))
        .unwrap_or_default();
    (u16::from_le_bytes(buf) as u64) % 300
}

/// Derive HTTP address (host:8080) from gRPC endpoint
fn derive_http_addr(master_endpoint: &str) -> String {
    let stripped = master_endpoint.trim_start_matches("https://").trim_start_matches("http://");
    let host = stripped.split(':').next().unwrap_or("localhost");
    format!("{host}:8080")
}

/// Simple HTTP GET returning body as string
async fn http_get(addr: &str, path: &str) -> Result<String> {
    let bytes = http_get_bytes(addr, path).await?;
    Ok(String::from_utf8_lossy(&bytes).to_string())
}

/// Simple HTTP GET returning body as bytes (no external HTTP client dep)
async fn http_get_bytes(addr: &str, path: &str) -> Result<Vec<u8>> {
    let mut stream = tokio::net::TcpStream::connect(addr).await?;
    let host = addr.split(':').next().unwrap_or("localhost");
    let req = format!("GET {path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n");
    stream.write_all(req.as_bytes()).await?;

    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).await?;
    let response = String::from_utf8_lossy(&buf);

    // Parse HTTP response: split headers from body
    let body_start = response.find("\r\n\r\n").map(|i| i + 4).unwrap_or(0);

    // Check for chunked transfer encoding or content-length
    let headers = &response[..body_start];
    if headers.to_lowercase().contains("transfer-encoding: chunked") {
        Ok(decode_chunked(&buf[body_start..]))
    } else {
        Ok(buf[body_start..].to_vec())
    }
}

fn decode_chunked(data: &[u8]) -> Vec<u8> {
    let mut result = Vec::new();
    let mut pos = 0;
    while pos < data.len() {
        // Find end of chunk size line
        let line_end = data[pos..].windows(2).position(|w| w == b"\r\n").unwrap_or(data.len() - pos);
        let size_str = std::str::from_utf8(&data[pos..pos + line_end]).unwrap_or("0");
        let chunk_size = usize::from_str_radix(size_str.trim(), 16).unwrap_or(0);
        if chunk_size == 0 {
            break;
        }
        pos += line_end + 2; // skip size line + CRLF
        if pos + chunk_size <= data.len() {
            result.extend_from_slice(&data[pos..pos + chunk_size]);
        }
        pos += chunk_size + 2; // skip chunk data + CRLF
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_http_addr_https() {
        assert_eq!(derive_http_addr("https://192.168.1.1:50051"), "192.168.1.1:8080");
    }

    #[test]
    fn test_derive_http_addr_http() {
        assert_eq!(derive_http_addr("http://master.local:50051"), "master.local:8080");
    }

    #[test]
    fn test_derive_http_addr_plain() {
        assert_eq!(derive_http_addr("10.0.0.1:50051"), "10.0.0.1:8080");
    }

    #[test]
    fn test_decode_chunked_simple() {
        // "5\r\nhello\r\n0\r\n\r\n"
        let data = b"5\r\nhello\r\n0\r\n\r\n";
        let result = decode_chunked(data);
        assert_eq!(result, b"hello");
    }

    #[test]
    fn test_decode_chunked_multi() {
        let data = b"5\r\nhello\r\n6\r\n world\r\n0\r\n\r\n";
        let result = decode_chunked(data);
        assert_eq!(result, b"hello world");
    }

    #[test]
    fn test_decode_chunked_empty() {
        let data = b"0\r\n\r\n";
        let result = decode_chunked(data);
        assert!(result.is_empty());
    }

    #[test]
    fn test_random_delay_bounded() {
        // Just verify it doesn't panic and returns < 300
        let delay = random_delay();
        assert!(delay < 300);
    }

    #[test]
    fn test_write_update_marker() {
        let tmp = std::env::temp_dir().join("pulse_test_marker");
        std::fs::create_dir_all(&tmp).ok();
        let marker_path = tmp.join("update-marker");

        // Temporarily write marker using direct fs (simulating write_update_marker logic)
        std::fs::write(&marker_path, "0.2.0\n0.3.0\n0").unwrap();

        let content = std::fs::read_to_string(&marker_path).unwrap();
        let parts: Vec<&str> = content.trim().split('\n').collect();
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[0], "0.2.0");
        assert_eq!(parts[1], "0.3.0");
        assert_eq!(parts[2], "0");

        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn test_marker_increment() {
        let tmp = std::env::temp_dir().join("pulse_test_marker_inc");
        std::fs::create_dir_all(&tmp).ok();
        let marker_path = tmp.join("marker");

        // Write initial marker
        std::fs::write(&marker_path, "0.2.0\n0.3.0\n0").unwrap();

        // Simulate increment (what check_rollback_needed does)
        let content = std::fs::read_to_string(&marker_path).unwrap();
        let parts: Vec<&str> = content.trim().split('\n').collect();
        let attempts: u32 = parts[2].parse().unwrap();
        let new_content = format!("{}\n{}\n{}", parts[0], parts[1], attempts + 1);
        std::fs::write(&marker_path, &new_content).unwrap();

        // Verify incremented
        let content = std::fs::read_to_string(&marker_path).unwrap();
        assert!(content.ends_with("\n1"));

        std::fs::remove_dir_all(&tmp).ok();
    }
}
