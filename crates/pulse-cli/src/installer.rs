use anyhow::{Result, bail};
use tokio::process::Command;
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

pub struct InstallOpts {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub key_file: Option<String>,
    pub github_token: String,
    pub github_repo: String,
    pub master_url: Option<String>,
    pub master_admin_token: Option<String>,
    pub register_token: Option<String>,
}

impl InstallOpts {
    /// Build the SSH destination string: user@host
    fn dest(&self) -> String {
        format!("{}@{}", self.user, self.host)
    }

    /// Build common SSH args: [-p port] [-i key]
    fn ssh_args(&self) -> Vec<String> {
        let mut args = vec![
            "-o".into(),
            "StrictHostKeyChecking=no".into(),
            "-o".into(),
            "BatchMode=yes".into(),
            "-p".into(),
            self.port.to_string(),
        ];
        if let Some(ref key) = self.key_file {
            args.push("-i".into());
            args.push(key.clone());
        }
        args
    }
}

/// Install pulse-master on a remote server.
pub async fn install_master(opts: InstallOpts, tx: mpsc::Sender<InstallEvent>) {
    let result = do_install_master(opts, &tx).await;
    let _ = tx.send(InstallEvent::Done(result)).await;
}

/// Install pulse-agent on a remote server.
pub async fn install_agent(opts: InstallOpts, tx: mpsc::Sender<InstallEvent>) {
    let result = do_install_agent(opts, &tx).await;
    let _ = tx.send(InstallEvent::Done(result)).await;
}

async fn do_install_master(opts: InstallOpts, tx: &mpsc::Sender<InstallEvent>) -> Result<InstallResult> {
    // Test SSH connection
    p(tx, "Testando conexão SSH...").await;
    ssh_exec(&opts, "echo ok").await?;
    p(tx, "✓ Conexão SSH OK (via ssh-agent/keychain)").await;

    // Detect arch
    let arch = ssh_exec(&opts, "uname -m").await?.trim().to_string();
    let arch_name = match arch.as_str() {
        "x86_64" | "amd64" => "x86_64",
        "aarch64" | "arm64" => "aarch64",
        _ => bail!("Arch não suportada: {arch}"),
    };
    p(tx, &format!("  Arch: linux/{arch_name}")).await;

    // Download binary from GitHub
    let asset = github::asset_name("master", arch_name);
    p(tx, &format!("Baixando {asset} do GitHub...")).await;
    let (version, binary) = github::download_release_binary(&opts.github_token, &opts.github_repo, &asset).await?;
    p(tx, &format!("✓ Download completo ({:.1} MB, {version})", binary.len() as f64 / 1_048_576.0)).await;

    // Upload binary via scp
    let tmp_local = format!("/tmp/pulse-master-upload-{}", std::process::id());
    tokio::fs::write(&tmp_local, &binary).await?;
    p(tx, "Uploading binário...").await;
    scp_upload(&opts, &tmp_local, "/tmp/pulse-master-upload").await?;
    tokio::fs::remove_file(&tmp_local).await.ok();
    ssh_exec(
        &opts,
        "sudo mv /tmp/pulse-master-upload /usr/local/bin/pulse-master && sudo chmod +x /usr/local/bin/pulse-master",
    )
    .await?;
    p(tx, "✓ Upload completo").await;

    // Create dirs + systemd
    p(tx, "Configurando systemd...").await;
    ssh_exec(&opts, "sudo mkdir -p /var/lib/pulse /etc/pulse").await?;

    let unit = "[Unit]\nDescription=Pulse Master\nAfter=network-online.target\nWants=network-online.target\n\n[Service]\nType=simple\nExecStart=/usr/local/bin/pulse-master\nRestart=always\nRestartSec=5\nEnvironment=RUST_LOG=info\nEnvironment=PULSE_LISTEN=0.0.0.0:50051\nWorkingDirectory=/var/lib/pulse\nNoNewPrivileges=yes\nProtectSystem=strict\nProtectHome=yes\nPrivateTmp=yes\nReadWritePaths=/var/lib/pulse /etc/pulse\n\n[Install]\nWantedBy=multi-user.target";
    ssh_exec(&opts, &format!("echo '{unit}' | sudo tee /etc/systemd/system/pulse-master.service > /dev/null")).await?;
    ssh_exec(&opts, "sudo systemctl daemon-reload && sudo systemctl enable --now pulse-master").await?;
    p(tx, "✓ pulse-master iniciado").await;

    // Wait and read tokens
    p(tx, "Aguardando tokens...").await;
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    let admin_token = ssh_exec(&opts, "sudo cat /var/lib/pulse/admin-token 2>/dev/null || echo ''")
        .await
        .unwrap_or_default()
        .trim()
        .to_string();
    let register_token = ssh_exec(&opts, "sudo cat /var/lib/pulse/register-token 2>/dev/null || echo ''")
        .await
        .unwrap_or_default()
        .trim()
        .to_string();

    // Health check
    let health = ssh_exec(&opts, "curl -sf http://localhost:8080/health 2>/dev/null || echo FAIL").await?;
    if health.contains("FAIL") {
        bail!("Master não respondeu no health check. Verifique: journalctl -u pulse-master");
    }

    let master_url = format!("http://{}:8080", opts.host);
    p(tx, &format!("✓ Master rodando em {master_url}")).await;
    p(tx, &format!("  Admin: {}", &admin_token[..admin_token.len().min(16)])).await;

    Ok(InstallResult { admin_token, register_token, master_url })
}

async fn do_install_agent(opts: InstallOpts, tx: &mpsc::Sender<InstallEvent>) -> Result<InstallResult> {
    let master_url = opts.master_url.as_deref().unwrap_or("");
    let mut register_token = opts.register_token.as_deref().unwrap_or("").to_string();

    // Auto-fetch register token from master if not provided
    if register_token.is_empty() && !master_url.is_empty() {
        p(tx, "Buscando register token do master...").await;
        let client = reqwest::Client::new();
        // Try with admin token first
        let url = format!("{master_url}/api/register-token");
        if let Ok(resp) = client
            .get(&url)
            .header("Authorization", format!("Bearer {}", opts.master_admin_token.as_deref().unwrap_or("")))
            .send()
            .await
        {
            if let Ok(body) = resp.json::<serde_json::Value>().await {
                if let Some(t) = body.get("register_token").and_then(|v| v.as_str()) {
                    register_token = t.to_string();
                    p(tx, "✓ Register token obtido do master").await;
                }
            }
        }
        // Fallback: try reading from master via SSH (if master is on same network)
        if register_token.is_empty() {
            bail!(
                "Não foi possível obter o register token.\n\
                 O master precisa estar rodando e acessível em {master_url}.\n\
                 Verifique se o master tem o endpoint /api/register-token."
            );
        }
    } else if register_token.is_empty() {
        bail!("Master URL necessária para instalar agent. Conecte a um master primeiro.");
    }

    p(tx, "Testando conexão SSH...").await;
    ssh_exec(&opts, "echo ok").await?;
    p(tx, "✓ Conexão SSH OK").await;

    let arch = ssh_exec(&opts, "uname -m").await?.trim().to_string();
    let arch_name = match arch.as_str() {
        "x86_64" | "amd64" => "x86_64",
        "aarch64" | "arm64" => "aarch64",
        _ => bail!("Arch não suportada: {arch}"),
    };
    p(tx, &format!("  Arch: linux/{arch_name}")).await;

    let asset = github::asset_name("agent", arch_name);
    p(tx, &format!("Baixando {asset}...")).await;
    let (version, binary) = github::download_release_binary(&opts.github_token, &opts.github_repo, &asset).await?;
    p(tx, &format!("✓ Download ({:.1} MB, {version})", binary.len() as f64 / 1_048_576.0)).await;

    let tmp_local = format!("/tmp/pulse-agent-upload-{}", std::process::id());
    tokio::fs::write(&tmp_local, &binary).await?;
    p(tx, "Uploading...").await;
    scp_upload(&opts, &tmp_local, "/tmp/pulse-agent-upload").await?;
    tokio::fs::remove_file(&tmp_local).await.ok();
    ssh_exec(
        &opts,
        "sudo mv /tmp/pulse-agent-upload /usr/local/bin/pulse-agent && sudo chmod +x /usr/local/bin/pulse-agent",
    )
    .await?;
    p(tx, "✓ Upload completo").await;

    // Config
    let grpc = master_url.replace("http://", "https://").replace(":8080", ":50051");
    let config =
        format!("master_endpoint = \\\"{grpc}\\\"\nmetrics_interval_ms = 10000\nauth_token = \\\"{register_token}\\\"");
    ssh_exec(&opts, "sudo mkdir -p /etc/pulse").await?;
    ssh_exec(&opts, &format!("printf '{config}' | sudo tee /etc/pulse/agent.toml > /dev/null")).await?;

    let unit = "[Unit]\nDescription=Pulse Agent\nAfter=network-online.target\nWants=network-online.target\n\n[Service]\nType=simple\nExecStart=/usr/local/bin/pulse-agent /etc/pulse/agent.toml\nRestart=always\nRestartSec=5\nEnvironment=RUST_LOG=info\nNoNewPrivileges=yes\nProtectSystem=strict\nProtectHome=yes\nPrivateTmp=yes\nReadWritePaths=/etc/pulse /var/lib/pulse\n\n[Install]\nWantedBy=multi-user.target";
    ssh_exec(&opts, &format!("echo '{unit}' | sudo tee /etc/systemd/system/pulse-agent.service > /dev/null")).await?;
    ssh_exec(&opts, "sudo systemctl daemon-reload && sudo systemctl enable --now pulse-agent").await?;

    p(tx, "✓ pulse-agent iniciado").await;
    p(tx, &format!("  Conectando a: {grpc}")).await;

    Ok(InstallResult {
        admin_token: String::new(),
        register_token: register_token.to_string(),
        master_url: master_url.to_string(),
    })
}

// ─── Helpers ──────────────────────────────────────────────────────

async fn p(tx: &mpsc::Sender<InstallEvent>, msg: &str) {
    tx.send(InstallEvent::Progress(msg.to_string())).await.ok();
}

/// Execute a command on the remote server using the system's `ssh` binary.
/// This respects ~/.ssh/config, Keychain, ssh-agent, ProxyJump, etc.
async fn ssh_exec(opts: &InstallOpts, cmd: &str) -> Result<String> {
    let mut args = opts.ssh_args();
    args.push(opts.dest());
    args.push(cmd.into());

    let output = Command::new("ssh").args(&args).output().await?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("SSH falhou: {}", stderr.trim());
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Upload a file using the system's `scp` binary.
async fn scp_upload(opts: &InstallOpts, local_path: &str, remote_path: &str) -> Result<()> {
    let mut args = vec![
        "-o".to_string(),
        "StrictHostKeyChecking=no".into(),
        "-o".to_string(),
        "BatchMode=yes".into(),
        "-P".to_string(),
        opts.port.to_string(),
    ];
    if let Some(ref key) = opts.key_file {
        args.push("-i".into());
        args.push(key.clone());
    }
    args.push(local_path.into());
    args.push(format!("{}:{}", opts.dest(), remote_path));

    let output = Command::new("scp").args(&args).output().await?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("SCP falhou: {}", stderr.trim());
    }
    Ok(())
}
