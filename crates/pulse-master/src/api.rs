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
    let rows = sqlx::query_as::<_, (String, String, String, String, String, String)>(
        "SELECT id, hostname, os, arch, agent_version, COALESCE(status,'online') FROM agents ORDER BY registered_at DESC",
    )
    .fetch_all(db.pool())
    .await;

    match rows {
        Ok(agents) => Json(json!({
            "agents": agents.iter().map(|a| json!({
                "id": a.0,
                "hostname": a.1,
                "os": a.2,
                "arch": a.3,
                "version": a.4,
                "status": a.5,
            })).collect::<Vec<_>>()
        })),
        Err(e) => Json(json!({"error": e.to_string()})),
    }
}

async fn get_metrics(State(db): State<Storage>, Path(agent_id): Path<String>) -> Json<Value> {
    let rows = sqlx::query_as::<_, (String, Option<i32>, Option<i64>, Option<i64>, Option<i64>)>(
        "SELECT time::text, cpu_total_bp, mem_used, mem_total, uptime_secs FROM metrics WHERE agent_id = $1 ORDER BY time DESC LIMIT 20",
    )
    .bind(&agent_id)
    .fetch_all(db.pool())
    .await;

    match rows {
        Ok(metrics) => Json(json!({
            "agent_id": agent_id,
            "metrics": metrics.iter().map(|m| json!({
                "time": m.0,
                "cpu_pct": m.1.map(|v| v as f64 / 100.0),
                "mem_used_mb": m.2.map(|v| v / 1_048_576),
                "mem_total_mb": m.3.map(|v| v / 1_048_576),
                "uptime_secs": m.4,
            })).collect::<Vec<_>>()
        })),
        Err(e) => Json(json!({"error": e.to_string()})),
    }
}

async fn get_sessions(State(db): State<Storage>, Path(agent_id): Path<String>) -> Json<Value> {
    // SSH sessions come from the latest metrics payload — stored as part of the gRPC stream
    // For now, return latest agent info + last_seen as proxy
    let row = sqlx::query_as::<_, (String, String, String)>(
        "SELECT hostname, COALESCE(status,'online'), last_seen_at::text FROM agents WHERE id = $1",
    )
    .bind(&agent_id)
    .fetch_optional(db.pool())
    .await;

    match row {
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
