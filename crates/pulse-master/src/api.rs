use crate::storage::Storage;
use axum::{
    Json, Router,
    extract::{Path, State},
};
use serde_json::{Value, json};

pub fn router(storage: Storage) -> Router {
    Router::new()
        .route("/health", axum::routing::get(health))
        .route("/api/agents", axum::routing::get(list_agents))
        .route("/api/agents/{id}/metrics", axum::routing::get(get_metrics))
        .route("/api/agents/{id}/sessions", axum::routing::get(get_sessions))
        .with_state(storage)
}

async fn health() -> Json<Value> {
    Json(json!({"status": "ok"}))
}

async fn list_agents(State(db): State<Storage>) -> Json<Value> {
    match db.list_agents().await {
        Ok(agents) => Json(json!({"agents": agents})),
        Err(e) => Json(json!({"error": e.to_string()})),
    }
}

async fn get_metrics(State(db): State<Storage>, Path(agent_id): Path<String>) -> Json<Value> {
    match db.get_agent_metrics(&agent_id).await {
        Ok(metrics) => Json(json!({"agent_id": agent_id, "metrics": metrics})),
        Err(e) => Json(json!({"error": e.to_string()})),
    }
}

async fn get_sessions(State(db): State<Storage>, Path(agent_id): Path<String>) -> Json<Value> {
    match db.get_agent_info(&agent_id).await {
        Ok(Some(r)) => Json(json!({
            "agent_id": agent_id,
            "hostname": r.0,
            "status": r.1,
            "last_seen": r.2,
            "note": "SSH sessions are streamed in real-time via gRPC. Query the metrics stream for live session data."
        })),
        Ok(None) => Json(json!({"error": "agent not found"})),
        Err(e) => Json(json!({"error": e.to_string()})),
    }
}
