use axum::{extract::{Path, State}, http::StatusCode, routing::{get, post, put}, Json, Router};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::limit::RequestBodyLimitLayer;
#[derive(Clone)]
pub struct AppState {
    pub manager: Arc<rivaldsend_core::manager::TransferManager>,
}
#[derive(Serialize)]
pub struct Health { pub status: String }
async fn health() -> Json<Health> { Json(Health { status: "ok".into() }) }
async fn negotiate(State(_s): State<AppState>, Json(req): Json<rivaldsend_proto::NegotiateRequest>) -> Result<Json<rivaldsend_proto::NegotiateResponse>, (StatusCode, String)> {
    rivaldsend_proto::negotiate(&req, "1.0.0", "2.0.0").map(Json).map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))
}
#[derive(Deserialize)]
pub struct CreateTransfer { pub manifest: rivaldsend_proto::TransferManifest }
async fn create_transfer(State(_s): State<AppState>, Json(body): Json<CreateTransfer>) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    rivaldsend_proto::validation::validate_manifest(&body.manifest).map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    Ok(Json(serde_json::json!({"transfer_id": body.manifest.transfer_id})))
}
async fn put_chunk(State(s): State<AppState>, Path(id): Path<String>, body: bytes::Bytes) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    if body.len() > rivaldsend_proto::limits::MAX_CHUNK_SIZE { return Err((StatusCode::PAYLOAD_TOO_LARGE, "chunk too large".into())); }
    let uuid = id.parse::<uuid::Uuid>().map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    if s.manager.status(&uuid).await.is_none() {
        return Err((StatusCode::NOT_FOUND, "transfer not found".into()));
    }
    let chunk_dir = rivaldsend_core::manager::TransferManager::default_partial_dir(uuid);
    tokio::fs::create_dir_all(&chunk_dir).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let offset_path = chunk_dir.join(format!("{}.chunk", uuid));
    tokio::fs::write(&offset_path, &body).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let ack = body.len() as u64;
    let resume_path = s.manager.resume_path(uuid);
    let mut state = s.manager.resume(uuid).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?.unwrap_or_else(|| rivaldsend_core::resume::ResumeState::new(uuid));
    state.add_ack(rivaldsend_proto::ChunkAck { offset: 0, size: ack as u32 });
    let _ = rivaldsend_core::resume::save(&resume_path, &state).await;
    Ok(Json(serde_json::json!({"ack_offset": ack})))
}
async fn resume_transfer(State(s): State<AppState>, Path(id): Path<String>) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let uuid = id.parse::<uuid::Uuid>().map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    let state = s.manager.resume(uuid).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(serde_json::to_value(state).unwrap_or(serde_json::Value::Null)))
}
async fn complete_transfer(Path(id): Path<String>, Json(_body): Json<serde_json::Value>) -> Json<serde_json::Value> {
    Json(serde_json::json!({"transfer_id": id, "status":"completed"}))
}
async fn cancel_transfer(State(s): State<AppState>, Path(id): Path<String>) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let uuid = id.parse::<uuid::Uuid>().map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    s.manager.cancel(uuid).await.map_err(|e| (StatusCode::NOT_FOUND, e.to_string()))?;
    Ok(Json(serde_json::json!({"transfer_id": uuid, "status":"cancelled"})))
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/v1/health", get(health))
        .route("/v1/negotiate", post(negotiate))
        .route("/v1/transfers", post(create_transfer))
        .route("/v1/transfers/:id/chunks", put(put_chunk))
        .route("/v1/transfers/:id/resume", post(resume_transfer))
        .route("/v1/transfers/:id/complete", post(complete_transfer))
        .route("/v1/transfers/:id", axum::routing::delete(cancel_transfer))
        .layer(ServiceBuilder::new().layer(RequestBodyLimitLayer::new(32 * 1024 * 1024)).layer(tower::limit::ConcurrencyLimitLayer::new(50)).layer(tower_http::trace::TraceLayer::new_for_http()))
        .with_state(state)
}
