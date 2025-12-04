pub mod dto;

use crate::api::dto::CreateTaskReq;
use crate::errors::AppError;
use crate::service::TaskService;
use axum::{
    Json, Router,
    extract::{Path, Request, State},
    http::{HeaderValue, StatusCode},
    routing::{delete, post},
};
use serde_json::{Value, json};
use sqlx::Row;
use tower_http::{
    request_id::{MakeRequestId, PropagateRequestIdLayer, RequestId, SetRequestIdLayer},
    trace::{DefaultOnRequest, DefaultOnResponse, TraceLayer},
};

use uuid::Uuid;

#[cfg(test)]
mod tests;
#[derive(Clone)]
pub struct AppState {
    pub service: TaskService,
}

#[derive(Clone, Copy)]
pub struct MakeUuidRequest;

impl MakeRequestId for MakeUuidRequest {
    fn make_request_id<B>(&mut self, _: &axum::http::Request<B>) -> Option<RequestId> {
        let uuid = Uuid::new_v4().to_string();

        let header_value =
            HeaderValue::from_str(&uuid).unwrap_or(HeaderValue::from_static("invalid-uuid"));

        Some(RequestId::new(header_value))
    }
}

/// Build the application router with all routes and middleware
///
/// # Arguments
///
/// * `service` - An instance of TaskService to handle business logic
///
/// # Returns
/// * `Router` - The configured Axum router
pub fn router(service: TaskService) -> Router {
    let state = AppState { service };

    let x_request_id = "x-request-id".parse::<axum::http::HeaderName>().unwrap();

    Router::new()
        .route("/tasks", post(create_task).get(list_tasks))
        .route("/tasks/{id}", delete(delete_task))
        .with_state(state)
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(|request: &Request<_>| {
                    let req_id = request
                        .extensions()
                        .get::<RequestId>()
                        .map(|id| id.header_value().to_str().unwrap_or("bad-ascii"))
                        .unwrap_or("unknown");

                    tracing::info_span!(
                        "http_request",
                        request_id = %req_id,
                        method = ?request.method(),
                        uri = ?request.uri(),
                    )
                })
                .on_request(DefaultOnRequest::new().level(tracing::Level::INFO))
                .on_response(DefaultOnResponse::new().level(tracing::Level::INFO)),
        )
        .layer(PropagateRequestIdLayer::new(x_request_id.clone()))
        .layer(SetRequestIdLayer::new(x_request_id, MakeUuidRequest))
}

/// Handler to create a new task
///
/// # Arguments
///
/// * `State(state)` - Application state containing the TaskService
/// * `Json(payload)` - JSON payload containing task creation details
///
/// # Errors
///
/// * `AppError` - If task creation fails (see TaskService::create_task for details)
async fn create_task(
    State(state): State<AppState>,
    Json(payload): Json<CreateTaskReq>,
) -> Result<Json<Value>, AppError> {
    let task_id = state.service.create_task(payload).await?;

    tracing::info!(%task_id, "Task Created Successfully");

    Ok(Json(json!({ "status": "created","id": task_id })))
}

/// Handler to delete a task by its ID
///
/// # Arguments
///
/// * `State(state)` - Application state containing the TaskService
/// * `Path(task_id)` - Path parameter containing the UUID of the task to delete
///
/// # Errors
///
/// * `AppError` - If task deletion fails (see TaskService::delete_task for details)
async fn delete_task(
    State(state): State<AppState>,
    Path(task_id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    state.service.delete_task(task_id).await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Handler to list all tasks
///
/// # Arguments
///
/// * `State(state)` - Application state containing the TaskService
///
/// # Errors
///
/// * `AppError` - If the database query fails or data cannot be retrieved
async fn list_tasks(State(state): State<AppState>) -> Result<Json<Value>, AppError> {
    let rows = sqlx::query("SELECT id,name,deleted_at FROM tasks")
        .fetch_all(state.service.get_pool())
        .await?;

    let tasks: Vec<Value> = rows
        .iter()
        .map(|row| {
            let id_res: Result<String, _> = row.try_get("id");

            let id_display = match id_res {
                Ok(id) => id,
                Err(_) => {
                    let uuid: Uuid = row.get("id");
                    uuid.to_string()
                }
            };

            let deleted: Option<String> = row.get("deleted_at");

            json!({
                "id": id_display,
                "name": row.try_get::<String, _>("name").unwrap_or_default(),
                "deleted_at": deleted,
            })
        })
        .collect();

    Ok(Json(json!(tasks)))
}
