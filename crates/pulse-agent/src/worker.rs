use anyhow::Result;
use pulse_proto::pb::{
    CommandRequest, CommandResponse, CommandResult, OutputChunk, output_chunk::Stream as OutputStream,
};
use tokio::io::AsyncReadExt;
use tokio::process::Command;
use tokio::sync::mpsc;
use tokio::time::{Duration, timeout};
use tracing::{info, warn};

pub async fn execute(req: CommandRequest, tx: mpsc::Sender<CommandResponse>) -> Result<()> {
    let request_id = req.request_id.clone();
    let timeout_secs = if req.timeout_secs > 0 { req.timeout_secs } else { 300 };

    let result = match req.command {
        Some(pulse_proto::pb::command_request::Command::Exec(exec)) => {
            execute_command(&request_id, &exec.program, &exec.args, &exec.working_dir, timeout_secs, &tx).await
        }
        Some(pulse_proto::pb::command_request::Command::Package(pkg)) => {
            let action = match pkg.action() {
                pulse_proto::pb::package_command::Action::Install => "install",
                pulse_proto::pb::package_command::Action::Remove => "remove",
                pulse_proto::pb::package_command::Action::Update => "update",
            };
            let args: Vec<String> = std::iter::once(action.to_string()).chain(pkg.packages.into_iter()).collect();
            execute_command(&request_id, "apt-get", &args, "", timeout_secs, &tx).await
        }
        Some(pulse_proto::pb::command_request::Command::File(file_cmd)) => {
            handle_file_command(&request_id, file_cmd, &tx).await;
            return Ok(());
        }
        Some(pulse_proto::pb::command_request::Command::LogTail(tail)) => {
            handle_log_tail(&request_id, tail, timeout_secs, &tx).await;
            return Ok(());
        }
        _ => {
            let _ = tx
                .send(CommandResponse {
                    request_id: request_id.clone(),
                    payload: Some(pulse_proto::pb::command_response::Payload::Result(CommandResult {
                        exit_code: -1,
                        timed_out: false,
                        error: "unsupported command type".into(),
                    })),
                })
                .await;
            return Ok(());
        }
    };

    let (exit_code, timed_out, error) = match result {
        Ok((code, to)) => (code, to, String::new()),
        Err(e) => (-1, false, e.to_string()),
    };

    let _ = tx
        .send(CommandResponse {
            request_id,
            payload: Some(pulse_proto::pb::command_response::Payload::Result(CommandResult {
                exit_code,
                timed_out,
                error,
            })),
        })
        .await;

    Ok(())
}

async fn execute_command(
    request_id: &str,
    program: &str,
    args: &[String],
    working_dir: &str,
    timeout_secs: u32,
    tx: &mpsc::Sender<CommandResponse>,
) -> Result<(i32, bool)> {
    info!(program, ?args, "executing command");

    let mut cmd = Command::new(program);
    cmd.args(args);
    if !working_dir.is_empty() {
        cmd.current_dir(working_dir);
    }
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    let mut child = cmd.spawn()?;
    let mut stdout = child.stdout.take().unwrap();
    let mut stderr = child.stderr.take().unwrap();

    let rid = request_id.to_string();
    let tx_out = tx.clone();
    let stdout_task = tokio::spawn(async move {
        let mut buf = [0u8; 4096];
        loop {
            match stdout.read(&mut buf).await {
                Ok(0) => break,
                Ok(n) => {
                    let _ = tx_out
                        .send(CommandResponse {
                            request_id: rid.clone(),
                            payload: Some(pulse_proto::pb::command_response::Payload::Output(OutputChunk {
                                stream: OutputStream::Stdout.into(),
                                data: buf[..n].to_vec(),
                            })),
                        })
                        .await;
                }
                Err(_) => break,
            }
        }
    });

    let rid = request_id.to_string();
    let tx_err = tx.clone();
    let stderr_task = tokio::spawn(async move {
        let mut buf = [0u8; 4096];
        loop {
            match stderr.read(&mut buf).await {
                Ok(0) => break,
                Ok(n) => {
                    let _ = tx_err
                        .send(CommandResponse {
                            request_id: rid.clone(),
                            payload: Some(pulse_proto::pb::command_response::Payload::Output(OutputChunk {
                                stream: OutputStream::Stderr.into(),
                                data: buf[..n].to_vec(),
                            })),
                        })
                        .await;
                }
                Err(_) => break,
            }
        }
    });

    let result = timeout(Duration::from_secs(timeout_secs.into()), child.wait()).await;

    let _ = stdout_task.await;
    let _ = stderr_task.await;

    match result {
        Ok(Ok(status)) => Ok((status.code().unwrap_or(-1), false)),
        Ok(Err(e)) => Err(e.into()),
        Err(_) => {
            warn!(program, "command timed out, killing");
            let _ = child.kill().await;
            Ok((-1, true))
        }
    }
}

async fn handle_file_command(request_id: &str, cmd: pulse_proto::pb::FileCommand, tx: &mpsc::Sender<CommandResponse>) {
    use pulse_proto::pb::file_command::Action;

    let result = match cmd.action() {
        Action::Read => match tokio::fs::read(&cmd.path).await {
            Ok(data) => {
                let _ = tx
                    .send(CommandResponse {
                        request_id: request_id.into(),
                        payload: Some(pulse_proto::pb::command_response::Payload::Output(OutputChunk {
                            stream: OutputStream::Stdout.into(),
                            data,
                        })),
                    })
                    .await;
                (0, false, String::new())
            }
            Err(e) => (-1, false, e.to_string()),
        },
        Action::Write => match tokio::fs::write(&cmd.path, &cmd.content).await {
            Ok(()) => (0, false, String::new()),
            Err(e) => (-1, false, e.to_string()),
        },
        Action::Append => {
            use tokio::io::AsyncWriteExt;
            match tokio::fs::OpenOptions::new().append(true).create(true).open(&cmd.path).await {
                Ok(mut f) => match f.write_all(&cmd.content).await {
                    Ok(()) => (0, false, String::new()),
                    Err(e) => (-1, false, e.to_string()),
                },
                Err(e) => (-1, false, e.to_string()),
            }
        }
    };

    let _ = tx
        .send(CommandResponse {
            request_id: request_id.into(),
            payload: Some(pulse_proto::pb::command_response::Payload::Result(CommandResult {
                exit_code: result.0,
                timed_out: result.1,
                error: result.2,
            })),
        })
        .await;
}

async fn handle_log_tail(
    request_id: &str,
    cmd: pulse_proto::pb::LogTailCommand,
    timeout_secs: u32,
    tx: &mpsc::Sender<CommandResponse>,
) {
    let lines = if cmd.lines > 0 { cmd.lines } else { 50 };
    let mut args = vec!["-n".to_string(), lines.to_string()];
    if cmd.follow {
        args.push("-f".to_string());
    }
    args.push(cmd.path);

    let result = execute_command(request_id, "tail", &args, "", timeout_secs, tx).await;

    let (exit_code, timed_out, error) = match result {
        Ok((code, to)) => (code, to, String::new()),
        Err(e) => (-1, false, e.to_string()),
    };

    let _ = tx
        .send(CommandResponse {
            request_id: request_id.into(),
            payload: Some(pulse_proto::pb::command_response::Payload::Result(CommandResult {
                exit_code,
                timed_out,
                error,
            })),
        })
        .await;
}
