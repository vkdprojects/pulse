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
        .route("/api/agents", axum::routing::get(list_agents))
        .route("/api/agents/{id}/metrics", axum::routing::get(get_metrics))
        .route("/api/agents/{id}/sessions", axum::routing::get(get_sessions))
        .route("/api/agents/{id}/connections", axum::routing::get(get_connections))
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

async fn list_agents(State(state): State<AppState>) -> Json<Value> {
    match state.storage.list_agents().await {
        Ok(agents) => Json(json!({"agents": agents})),
        Err(e) => Json(json!({"error": e.to_string()})),
    }
}

async fn get_metrics(State(state): State<AppState>, Path(agent_id): Path<String>) -> Json<Value> {
    match state.storage.get_agent_metrics(&agent_id).await {
        Ok(metrics) => Json(json!({"agent_id": agent_id, "metrics": metrics})),
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
