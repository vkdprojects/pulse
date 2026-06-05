use anyhow::{Result, bail};
use russh::keys::*;
use russh::*;
use std::sync::Arc;
use tokio::sync::mpsc;

use crate::github;

pub enum InstallEvent {
    Progress(String),
    Done(Result<InstallResult>),
}

pub struct InstallResult {
    pub admin_token: String,
    pub register_token: String,
    pub master_url: String,
}

struct SshHandler;

#[async_trait::async_trait]
impl client::Handler for SshHandler {
    type Error = anyhow::Error;
    async fn check_server_key(&mut self, _key: &key::PublicKey) -> Result<bool, Self::Error> {
        Ok(true) // TOFU
    }
}

pub struct InstallOpts {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: String,
    pub github_token: String,
    pub github_repo: String,
    pub master_url: Option<String>, // None = installing master, Some = agent needs this
    pub register_token: Option<String>, // For agent installs
}

/// Install pulse-master on a remote server via SSH.
pub async fn install_master(opts: InstallOpts, tx: mpsc::Sender<InstallEvent>) {
    let result = do_install_master(opts, &tx).await;
    let _ = tx.send(InstallEvent::Done(result)).await;
}

/// Install pulse-agent on a remote server via SSH.
pub async fn install_agent(opts: InstallOpts, tx: mpsc::Sender<InstallEvent>) {
    let result = do_install_agent(opts, &tx).await;
    let _ = tx.send(InstallEvent::Done(result)).await;
}

async fn do_install_master(opts: InstallOpts, tx: &mpsc::Sender<InstallEvent>) -> Result<InstallResult> {
    let session = connect_ssh(&opts, tx).await?;

    // Detect arch
    let arch = detect_arch(&session, tx).await?;

    // Download binary from GitHub
    let asset = github::asset_name("master", &arch);
    tx.send(InstallEvent::Progress(format!("Baixando {asset} do GitHub..."))).await.ok();
    let (version, binary) = github::download_release_binary(&opts.github_token, &opts.github_repo, &asset).await?;
    tx.send(InstallEvent::Progress(format!("✓ Download completo ({} MB, {version})", binary.len() / 1_048_576)))
        .await
        .ok();

    // Upload via SFTP
    upload_binary(&session, &binary, "/usr/local/bin/pulse-master", tx).await?;

    // Create directories
    exec(&session, "sudo mkdir -p /var/lib/pulse /etc/pulse").await?;

    // Create systemd service
    let unit = r#"[Unit]
Description=Vixpi Pulse Master
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
ExecStart=/usr/local/bin/pulse-master
Restart=always
RestartSec=5
Environment=RUST_LOG=info
Environment=PULSE_LISTEN=0.0.0.0:50051
WorkingDirectory=/var/lib/pulse
NoNewPrivileges=yes
ProtectSystem=strict
ProtectHome=yes
PrivateTmp=yes
ReadWritePaths=/var/lib/pulse /etc/pulse

[Install]
WantedBy=multi-user.target"#;

    tx.send(InstallEvent::Progress("Criando serviço systemd...".into())).await.ok();
    exec(&session, &format!("echo '{unit}' | sudo tee /etc/systemd/system/pulse-master.service > /dev/null")).await?;
    exec(&session, "sudo systemctl daemon-reload").await?;
    exec(&session, "sudo systemctl enable --now pulse-master").await?;

    // Wait for master to generate tokens
    tx.send(InstallEvent::Progress("Aguardando master iniciar...".into())).await.ok();
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    // Read tokens
    let admin_token = exec(&session, "sudo cat /var/lib/pulse/admin-token 2>/dev/null").await?.trim().to_string();
    let register_token = exec(&session, "sudo cat /var/lib/pulse/register-token 2>/dev/null").await?.trim().to_string();

    let master_url = format!("http://{}:8080", opts.host);

    // Verify
    let health = exec(&session, "curl -sf http://localhost:8080/health 2>/dev/null || echo FAIL").await?;
    if health.contains("FAIL") {
        bail!("Master não respondeu no health check");
    }

    tx.send(InstallEvent::Progress("✓ pulse-master rodando!".into())).await.ok();
    tx.send(InstallEvent::Progress(format!("  URL: {master_url}"))).await.ok();
    tx.send(InstallEvent::Progress(format!("  Admin token: {}...", &admin_token[..20.min(admin_token.len())])))
        .await
        .ok();

    session.disconnect(Disconnect::ByApplication, "", "English").await.ok();

    Ok(InstallResult { admin_token, register_token, master_url })
}

async fn do_install_agent(opts: InstallOpts, tx: &mpsc::Sender<InstallEvent>) -> Result<InstallResult> {
    let master_url = opts.master_url.as_deref().unwrap_or("http://localhost:8080");
    let register_token = opts.register_token.as_deref().unwrap_or("");

    if register_token.is_empty() {
        bail!("Register token necessário para instalar agent");
    }

    let session = connect_ssh(&opts, tx).await?;
    let arch = detect_arch(&session, tx).await?;

    // Download agent binary
    let asset = github::asset_name("agent", &arch);
    tx.send(InstallEvent::Progress(format!("Baixando {asset} do GitHub..."))).await.ok();
    let (version, binary) = github::download_release_binary(&opts.github_token, &opts.github_repo, &asset).await?;
    tx.send(InstallEvent::Progress(format!("✓ Download ({} MB, {version})", binary.len() / 1_048_576))).await.ok();

    // Upload
    upload_binary(&session, &binary, "/usr/local/bin/pulse-agent", tx).await?;

    // Create config
    // Convert HTTP master URL to gRPC endpoint (https for TLS bootstrap)
    let grpc_endpoint = master_url.replace("http://", "https://").replace(":8080", ":50051");
    let config_toml = format!(
        "master_endpoint = \"{grpc_endpoint}\"\nmetrics_interval_ms = 10000\nauth_token = \"{register_token}\"\n"
    );

    exec(&session, "sudo mkdir -p /etc/pulse").await?;
    exec(&session, &format!("echo '{config_toml}' | sudo tee /etc/pulse/agent.toml > /dev/null")).await?;

    // Systemd service
    let unit = r#"[Unit]
Description=Vixpi Pulse Agent
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
ExecStart=/usr/local/bin/pulse-agent /etc/pulse/agent.toml
Restart=always
RestartSec=5
Environment=RUST_LOG=info
NoNewPrivileges=yes
ProtectSystem=strict
ProtectHome=yes
PrivateTmp=yes
ReadWritePaths=/etc/pulse /var/lib/pulse

[Install]
WantedBy=multi-user.target"#;

    tx.send(InstallEvent::Progress("Criando serviço systemd...".into())).await.ok();
    exec(&session, &format!("echo '{unit}' | sudo tee /etc/systemd/system/pulse-agent.service > /dev/null")).await?;
    exec(&session, "sudo systemctl daemon-reload").await?;
    exec(&session, "sudo systemctl enable --now pulse-agent").await?;

    tx.send(InstallEvent::Progress("✓ pulse-agent rodando!".into())).await.ok();
    tx.send(InstallEvent::Progress(format!("  Conectando a: {grpc_endpoint}"))).await.ok();

    session.disconnect(Disconnect::ByApplication, "", "English").await.ok();

    Ok(InstallResult {
        admin_token: String::new(),
        register_token: register_token.to_string(),
        master_url: master_url.to_string(),
    })
}

// ─── SSH Helpers ──────────────────────────────────────────────────

async fn connect_ssh(opts: &InstallOpts, tx: &mpsc::Sender<InstallEvent>) -> Result<client::Handle<SshHandler>> {
    tx.send(InstallEvent::Progress(format!("Conectando {}@{}:{}...", opts.user, opts.host, opts.port))).await.ok();

    let config = Arc::new(client::Config::default());
    let mut session = client::connect(config, (&*opts.host, opts.port), SshHandler).await?;

    let auth = session.authenticate_password(&opts.user, &opts.password).await?;
    if !auth {
        bail!("Autenticação SSH falhou");
    }
    tx.send(InstallEvent::Progress("✓ Conectado via SSH".into())).await.ok();
    Ok(session)
}

async fn detect_arch(session: &client::Handle<SshHandler>, tx: &mpsc::Sender<InstallEvent>) -> Result<String> {
    let output = exec(session, "uname -m").await?;
    let arch = match output.trim() {
        "x86_64" | "amd64" => "x86_64",
        "aarch64" | "arm64" => "aarch64",
        other => bail!("Arch não suportada: {other}"),
    };
    tx.send(InstallEvent::Progress(format!("  Arch: linux/{arch}"))).await.ok();
    Ok(arch.into())
}

async fn upload_binary(
    session: &client::Handle<SshHandler>,
    data: &[u8],
    remote_path: &str,
    tx: &mpsc::Sender<InstallEvent>,
) -> Result<()> {
    tx.send(InstallEvent::Progress(format!("Uploading para {remote_path}..."))).await.ok();

    // Write to /tmp first (no sudo needed), then sudo mv
    let tmp = format!("/tmp/pulse-upload-{}", std::process::id());
    let channel = session.channel_open_session().await?;
    channel.request_subsystem(true, "sftp").await?;

    let sftp = russh_sftp::client::SftpSession::new(channel.into_stream()).await?;
    let mut file = sftp.create(&tmp).await?;
    use tokio::io::AsyncWriteExt;
    file.write_all(data).await?;
    file.shutdown().await?;
    drop(sftp);

    // Move to final path with sudo + chmod
    exec(session, &format!("sudo mv {tmp} {remote_path} && sudo chmod +x {remote_path}")).await?;
    tx.send(InstallEvent::Progress(format!("✓ Upload completo ({} MB)", data.len() / 1_048_576))).await.ok();
    Ok(())
}

async fn exec(session: &client::Handle<SshHandler>, cmd: &str) -> Result<String> {
    let mut channel = session.channel_open_session().await?;
    channel.exec(true, cmd).await?;

    let mut output = String::new();
    loop {
        let Some(msg) = channel.wait().await else { break };
        match msg {
            ChannelMsg::Data { ref data } => output.push_str(&String::from_utf8_lossy(data)),
            ChannelMsg::ExtendedData { ref data, .. } => output.push_str(&String::from_utf8_lossy(data)),
            ChannelMsg::ExitStatus { .. } => break,
            _ => {}
        }
    }
    Ok(output)
}
