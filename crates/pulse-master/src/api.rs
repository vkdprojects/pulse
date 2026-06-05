use crate::auth::{AdminToken, require_admin};
use crate::storage::Storage;
use crate::updates::UpdateState;
use axum::{
    Extension, Json, Router,
    extract::{Path, State},
    middleware,
};
use serde_json::{Value, json};
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub storage: Storage,
    pub ca_pem: Option<Arc<String>>,
    pub update_state: UpdateState,
}

pub fn router(storage: Storage) -> Router<()> {
    router_full(storage, None, None, UpdateState::new("data"))
}

pub fn router_with_ca(storage: Storage, ca_pem: Option<String>) -> Router<()> {
    router_full(storage, ca_pem, None, UpdateState::new("data"))
}

pub fn router_full(
    storage: Storage,
    ca_pem: Option<String>,
    admin_token: Option<String>,
    update_state: UpdateState,
) -> Router<()> {
    let state = AppState { storage, ca_pem: ca_pem.map(Arc::new), update_state };

    // Admin-protected routes
    let admin_routes = Router::new()
        .route("/api/agents/{id}", axum::routing::delete(delete_agent))
        .route("/api/agents/{id}/name", axum::routing::put(rename_agent))
        .route("/api/agents/{id}/exec", axum::routing::post(post_exec))
        .route("/api/agents/{id}/file", axum::routing::post(post_file))
        .route("/api/register-token", axum::routing::get(get_register_token))
        .route("/api/updates/upload", axum::routing::post(crate::updates::upload_binary))
        .route("/api/updates/rollout", axum::routing::post(crate::updates::set_rollout))
        .layer(axum::extract::DefaultBodyLimit::max(50 * 1024 * 1024)) // 50MB for binary uploads
        .layer(middleware::from_fn(require_admin));

    // Public-ish routes (read access — optionally protected via PULSE_API_PUBLIC)
    let read_routes = Router::new()
        .route("/health", axum::routing::get(health))
        .route("/api/ca", axum::routing::get(get_ca))
        .route("/api/overview", axum::routing::get(get_overview))
        .route("/api/agents", axum::routing::get(list_agents))
        .route("/api/agents/{id}/metrics", axum::routing::get(get_metrics))
        .route("/api/agents/{id}/metrics/latest", axum::routing::get(get_metrics_latest))
        .route("/api/agents/{id}/disks", axum::routing::get(get_disks))
        .route("/api/agents/{id}/network", axum::routing::get(get_network))
        .route("/api/agents/{id}/containers", axum::routing::get(get_containers))
        .route("/api/agents/{id}/processes", axum::routing::get(get_processes))
        .route("/api/agents/{id}/gpu", axum::routing::get(get_gpu))
        .route("/api/agents/{id}/sessions", axum::routing::get(get_sessions))
        .route("/api/agents/{id}/connections", axum::routing::get(get_connections))
        .route("/api/alerts", axum::routing::get(get_alerts))
        .route("/api/updates/check", axum::routing::get(crate::updates::check_update))
        .route("/api/updates/download", axum::routing::get(crate::updates::download_binary))
        .route("/api/updates/rollout", axum::routing::get(crate::updates::get_rollout));

    let token_ext = admin_token.map(|t| AdminToken(Arc::new(t)));

    let mut app = Router::new().merge(admin_routes).merge(read_routes).with_state(state);

    if let Some(ext) = token_ext {
        app = app.layer(Extension(ext));
    }

    app
}

async fn health() -> Json<Value> {
    Json(json!({"status": "ok"}))
}

async fn get_ca(State(state): State<AppState>) -> axum::response::Response {
    use axum::response::IntoResponse;
    match &state.ca_pem {
        Some(pem) => {
            ([(axum::http::header::CONTENT_TYPE, "application/x-pem-file")], pem.as_str().to_string()).into_response()
        }
        None => (axum::http::StatusCode::NOT_FOUND, "TLS not configured").into_response(),
    }
}

async fn get_overview(State(state): State<AppState>) -> Json<Value> {
    match state.storage.get_overview_data().await {
        Ok(agents) => {
            let total = agents.len();
            let online = agents.iter().filter(|a| a.get("status").and_then(|v| v.as_str()) == Some("online")).count();
            Json(json!({"total_agents": total, "online": online, "offline": total - online, "agents": agents}))
        }
        Err(e) => Json(json!({"error": e.to_string()})),
    }
}

async fn list_agents(State(state): State<AppState>) -> Json<Value> {
    match state.storage.list_agents().await {
        Ok(agents) => Json(json!({"agents": agents})),
        Err(e) => Json(json!({"error": e.to_string()})),
    }
}

async fn delete_agent(State(state): State<AppState>, Path(agent_id): Path<String>) -> Json<Value> {
    match state.storage.delete_agent(&agent_id).await {
        Ok(()) => Json(json!({"deleted": true, "agent_id": agent_id})),
        Err(e) => Json(json!({"error": e.to_string()})),
    }
}

async fn get_metrics(State(state): State<AppState>, Path(agent_id): Path<String>) -> Json<Value> {
    match state.storage.get_agent_metrics(&agent_id).await {
        Ok(metrics) => Json(json!({"agent_id": agent_id, "metrics": metrics})),
        Err(e) => Json(json!({"error": e.to_string()})),
    }
}

async fn get_metrics_latest(State(state): State<AppState>, Path(agent_id): Path<String>) -> Json<Value> {
    match state.storage.get_latest_metric(&agent_id).await {
        Ok(Some(m)) => Json(m),
        Ok(None) => Json(json!({"error": "no metrics found"})),
        Err(e) => Json(json!({"error": e.to_string()})),
    }
}

async fn get_disks(State(state): State<AppState>, Path(agent_id): Path<String>) -> Json<Value> {
    match state.storage.get_agent_disks(&agent_id).await {
        Ok(disks) => Json(json!({"agent_id": agent_id, "disks": disks})),
        Err(e) => Json(json!({"error": e.to_string()})),
    }
}

async fn get_network(State(state): State<AppState>, Path(agent_id): Path<String>) -> Json<Value> {
    match state.storage.get_agent_network(&agent_id).await {
        Ok(interfaces) => Json(json!({"agent_id": agent_id, "interfaces": interfaces})),
        Err(e) => Json(json!({"error": e.to_string()})),
    }
}

async fn get_containers(State(state): State<AppState>, Path(agent_id): Path<String>) -> Json<Value> {
    match state.storage.get_agent_containers(&agent_id).await {
        Ok(containers) => Json(json!({"agent_id": agent_id, "containers": containers})),
        Err(e) => Json(json!({"error": e.to_string()})),
    }
}

async fn get_processes(State(state): State<AppState>, Path(agent_id): Path<String>) -> Json<Value> {
    match state.storage.get_agent_processes(&agent_id).await {
        Ok(processes) => Json(json!({"agent_id": agent_id, "processes": processes})),
        Err(e) => Json(json!({"error": e.to_string()})),
    }
}

async fn get_gpu(State(state): State<AppState>, Path(agent_id): Path<String>) -> Json<Value> {
    match state.storage.get_agent_gpu(&agent_id).await {
        Ok(gpus) => Json(json!({"agent_id": agent_id, "gpus": gpus})),
        Err(e) => Json(json!({"error": e.to_string()})),
    }
}

async fn get_sessions(State(state): State<AppState>, Path(agent_id): Path<String>) -> Json<Value> {
    match state.storage.get_agent_info(&agent_id).await {
        Ok(Some(r)) => Json(json!({
            "agent_id": agent_id,
            "hostname": r.0,
            "status": r.1,
            "last_seen": r.2,
        })),
        Ok(None) => Json(json!({"error": "agent not found"})),
        Err(e) => Json(json!({"error": e.to_string()})),
    }
}

async fn get_connections(State(state): State<AppState>, Path(agent_id): Path<String>) -> Json<Value> {
    match state.storage.get_agent_connections(&agent_id).await {
        Ok(conns) => Json(json!({"agent_id": agent_id, "connections": conns})),
        Err(e) => Json(json!({"error": e.to_string()})),
    }
}

async fn rename_agent(
    State(state): State<AppState>,
    Path(agent_id): Path<String>,
    Json(body): Json<Value>,
) -> Json<Value> {
    let Some(new_name) = body.get("name").and_then(|v| v.as_str()) else {
        return Json(json!({"error": "name field required"}));
    };
    match &state.storage {
        crate::storage::Storage::Postgres(pool) => {
            sqlx::query("UPDATE agents SET hostname = $2 WHERE id = $1")
                .bind(&agent_id)
                .bind(new_name)
                .execute(pool)
                .await
                .ok();
        }
        crate::storage::Storage::Sqlite(pool) => {
            sqlx::query("UPDATE agents SET hostname = $2 WHERE id = $1")
                .bind(&agent_id)
                .bind(new_name)
                .execute(pool)
                .await
                .ok();
        }
    }
    Json(json!({"ok": true, "agent_id": agent_id, "name": new_name}))
}

async fn get_register_token() -> Json<Value> {
    let paths = ["/var/lib/pulse/register-token", "data/register-token"];
    let mut token = String::new();
    for p in paths {
        if let Ok(t) = tokio::fs::read_to_string(p).await {
            token = t.trim().to_string();
            break;
        }
    }
    Json(json!({"register_token": token}))
}

/// Execute a command. Currently runs on the master server (useful for master-local ops).
/// TODO: Route to agent via gRPC CommandChannel for true remote exec.
/// Protected by admin token (admin_routes).
async fn post_exec(
    State(_state): State<AppState>,
    Path(agent_id): Path<String>,
    Json(body): Json<Value>,
) -> Json<Value> {
    let Some(command_str) = body.get("command").and_then(|v| v.as_str()) else {
        return Json(json!({"error": "command field required"}));
    };

    // Parse command string into program + args
    let parts: Vec<&str> = command_str.split_whitespace().collect();
    let (program, args) = match parts.split_first() {
        Some((p, a)) => (*p, a.to_vec()),
        None => return Json(json!({"error": "empty command"})),
    };

    let timeout = body.get("timeout_secs").and_then(|v| v.as_u64()).unwrap_or(30) as u32;

    // Execute locally via tokio::process (for now — future: route to agent via gRPC CommandChannel)
    use tokio::process::Command;
    let result = tokio::time::timeout(std::time::Duration::from_secs(timeout.into()), async {
        Command::new(program).args(&args).output().await
    })
    .await;

    match result {
        Ok(Ok(output)) => Json(json!({
            "agent_id": agent_id,
            "exit_code": output.status.code().unwrap_or(-1),
            "stdout": String::from_utf8_lossy(&output.stdout),
            "stderr": String::from_utf8_lossy(&output.stderr),
            "timed_out": false,
        })),
        Ok(Err(e)) => Json(json!({"error": e.to_string()})),
        Err(_) => Json(json!({"agent_id": agent_id, "exit_code": -1, "timed_out": true, "error": "command timed out"})),
    }
}

async fn post_file(Path(_agent_id): Path<String>) -> axum::response::Response {
    use axum::response::IntoResponse;
    (
        axum::http::StatusCode::NOT_IMPLEMENTED,
        Json(json!({"error": "Remote file ops via REST API coming soon. Use gRPC command channel directly."})),
    )
        .into_response()
}

async fn get_alerts(State(state): State<AppState>) -> Json<Value> {
    match state.storage.get_alerts_history().await {
        Ok(alerts) => Json(json!({"alerts": alerts})),
        Err(e) => Json(json!({"error": e.to_string()})),
    }
}
