use anyhow::{Result, bail};
use russh::keys::*;
use russh::*;
use std::sync::Arc;
use tokio::sync::mpsc;

pub enum SshEvent {
    Log(String),
    Done(Result<String>),
}

struct SshClient;

#[async_trait::async_trait]
impl client::Handler for SshClient {
    type Error = anyhow::Error;

    async fn check_server_key(&mut self, _server_public_key: &key::PublicKey) -> Result<bool, Self::Error> {
        Ok(true)
    }
}

pub async fn provision_master(host: &str, port: u16, user: &str, password: &str, tx: mpsc::Sender<SshEvent>) {
    let result = do_provision(host, port, user, password, None, None, &tx).await;
    let _ = tx.send(SshEvent::Done(result)).await;
}

pub async fn provision_agent(
    host: &str,
    port: u16,
    user: &str,
    password: &str,
    master_url: &str,
    register_token: &str,
    tx: mpsc::Sender<SshEvent>,
) {
    let result = do_provision(host, port, user, password, Some(master_url), Some(register_token), &tx).await;
    let _ = tx.send(SshEvent::Done(result)).await;
}

async fn do_provision(
    host: &str,
    port: u16,
    user: &str,
    password: &str,
    master_url: Option<&str>,
    register_token: Option<&str>,
    tx: &mpsc::Sender<SshEvent>,
) -> Result<String> {
    tx.send(SshEvent::Log(format!("Conectando a {user}@{host}:{port}..."))).await.ok();

    let config = client::Config::default();
    let config = Arc::new(config);
    let sh = SshClient;
    let mut session = client::connect(config, (host, port), sh).await?;

    tx.send(SshEvent::Log("Autenticando...".into())).await.ok();
    let auth = session.authenticate_password(user, password).await?;
    if !auth {
        bail!("Autenticação falhou");
    }
    tx.send(SshEvent::Log("✓ Autenticado".into())).await.ok();

    // Build install command
    let cmd = if let (Some(url), Some(token)) = (master_url, register_token) {
        format!(
            "curl -sfL https://raw.githubusercontent.com/gcnsolucoes/vixpi-pulse/main/dist/install-pulse.sh | PULSE_MASTER={url} PULSE_TOKEN={token} sudo sh -"
        )
    } else {
        "curl -sfL https://raw.githubusercontent.com/gcnsolucoes/vixpi-pulse/main/dist/install-pulse.sh | sudo sh -"
            .into()
    };

    tx.send(SshEvent::Log(format!("$ {}...", &cmd[..60.min(cmd.len())]))).await.ok();
    let output = exec_cmd(&session, &cmd).await?;
    for line in output.lines().filter(|l| !l.is_empty()) {
        tx.send(SshEvent::Log(format!("  {line}"))).await.ok();
    }

    // Get tokens
    let token = exec_cmd(&session, "sudo cat /var/lib/pulse/admin-token 2>/dev/null || echo ''").await?;
    tx.send(SshEvent::Log("✓ Instalação completa!".into())).await.ok();

    session.disconnect(Disconnect::ByApplication, "", "English").await?;
    Ok(token.trim().to_string())
}

async fn exec_cmd(session: &client::Handle<SshClient>, cmd: &str) -> Result<String> {
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
