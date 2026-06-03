pub mod ca;
pub mod service;
pub mod storage;

use anyhow::Result;
use ca::CaBundle;

pub fn gen_dev_certs(output_dir: &str) -> Result<()> {
    let ca = CaBundle::new()?;
    let server = ca.issue_server_cert(&["localhost", "::1", "127.0.0.1", "master", "pulse-master"])?;
    let client = ca.issue_client_cert("dev-agent")?;

    std::fs::create_dir_all(output_dir)?;
    std::fs::write(format!("{output_dir}/ca.pem"), &ca.ca_cert_pem)?;
    std::fs::write(format!("{output_dir}/server.pem"), &server.cert_pem)?;
    std::fs::write(format!("{output_dir}/server-key.pem"), &server.key_pem)?;
    std::fs::write(format!("{output_dir}/client.pem"), &client.cert_pem)?;
    std::fs::write(format!("{output_dir}/client-key.pem"), &client.key_pem)?;

    Ok(())
}
