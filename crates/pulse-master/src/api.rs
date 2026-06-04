use crate::storage::Storage;
use axum::{
    Json, Router,
    extract::{Path, State},
};
use serde_json::{Value, json};
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub storage: Storage,
    pub ca_pem: Option<Arc<String>>,
}

pub fn router(storage: Storage) -> Router<()> {
    router_with_ca(storage, None)
}

pub fn router_with_ca(storage: Storage, ca_pem: Option<String>) -> Router<()> {
    let state = AppState { storage, ca_pem: ca_pem.map(Arc::new) };
    Router::new()
        .route("/health", axum::routing::get(health))
        .route("/api/ca", axum::routing::get(get_ca))
        .route("/api/overview", axum::routing::get(get_overview))
        .route("/api/agents", axum::routing::get(list_agents))
        .route("/api/agents/{id}", axum::routing::delete(delete_agent))
        .route("/api/agents/{id}/metrics", axum::routing::get(get_metrics))
        .route("/api/agents/{id}/metrics/latest", axum::routing::get(get_metrics_latest))
        .route("/api/agents/{id}/disks", axum::routing::get(get_disks))
        .route("/api/agents/{id}/network", axum::routing::get(get_network))
        .route("/api/agents/{id}/containers", axum::routing::get(get_containers))
        .route("/api/agents/{id}/processes", axum::routing::get(get_processes))
        .route("/api/agents/{id}/gpu", axum::routing::get(get_gpu))
        .route("/api/agents/{id}/sessions", axum::routing::get(get_sessions))
        .route("/api/agents/{id}/connections", axum::routing::get(get_connections))
        .route("/api/agents/{id}/exec", axum::routing::post(post_exec))
        .route("/api/agents/{id}/file", axum::routing::post(post_file))
        .route("/api/alerts", axum::routing::get(get_alerts))
        .with_state(state)
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

async fn post_exec(Path(_agent_id): Path<String>) -> axum::response::Response {
    use axum::response::IntoResponse;
    (
        axum::http::StatusCode::NOT_IMPLEMENTED,
        Json(json!({"error": "Remote exec via REST API coming soon. Use gRPC command channel directly."})),
    )
        .into_response()
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
