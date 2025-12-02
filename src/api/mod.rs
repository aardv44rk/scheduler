pub mod dto;

use crate::api::dto::CreateTaskReq;
use crate::errors::AppError;
use crate::service::TaskService;
use axum::{Json, Router, extract::State, routing::post};
use serde_json::{Value, json};

#[derive(Clone)]
pub struct AppState {
    pub service: TaskService,
}

pub fn router(service: TaskService) -> Router {
    let state = AppState { service };

    Router::new()
        .route("/tasks", post(create_task))
        .with_state(state)
}

async fn create_task(
    State(state): State<AppState>,
    Json(payload): Json<CreateTaskReq>,
) -> Result<Json<Value>, AppError> {
    let task_id = state.service.create_task(payload).await?;

    Ok(Json(json!({ "status": "created","id": task_id })))
}
