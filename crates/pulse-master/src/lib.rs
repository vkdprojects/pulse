pub mod alerts;
pub mod api;
pub mod ca;
pub mod heartbeat;
pub mod service;
pub mod storage;

use anyhow::Result;
use ca::CaBundle;

pub fn gen_dev_certs(output_dir: &str) -> Result<()> {
    let ca = CaBundle::new()?;

    let mut sans: Vec<String> =
        vec!["localhost".into(), "::1".into(), "127.0.0.1".into(), "master".into(), "pulse-master".into()];
    if let Ok(extra) = std::env::var("PULSE_EXTRA_SAN") {
        for s in extra.split(',') {
            let s = s.trim();
            if !s.is_empty() {
                sans.push(s.to_string());
            }
        }
    }
    let san_refs: Vec<&str> = sans.iter().map(|s| s.as_str()).collect();
    let server = ca.issue_server_cert(&san_refs)?;
    let client = ca.issue_client_cert("dev-agent")?;

    std::fs::create_dir_all(output_dir)?;
    std::fs::write(format!("{output_dir}/ca.pem"), &ca.ca_cert_pem)?;
    std::fs::write(format!("{output_dir}/server.pem"), &server.cert_pem)?;
    std::fs::write(format!("{output_dir}/server-key.pem"), &server.key_pem)?;
    std::fs::write(format!("{output_dir}/client.pem"), &client.cert_pem)?;
    std::fs::write(format!("{output_dir}/client-key.pem"), &client.key_pem)?;

    Ok(())
}
