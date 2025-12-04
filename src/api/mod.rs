pub mod dto;

use crate::api::dto::{CreateTaskReq, TaskSummaryResponse};
use crate::errors::AppError;
use crate::service::TaskService;
use axum::{
    Json, Router,
    extract::{Path, Request, State},
    http::{HeaderValue, StatusCode},
    routing::{delete, post},
};
use serde_json::{Value, json};
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
/// * `AppError` - If listing tasks fails (see TaskService::list_tasks for details)
async fn list_tasks(
    State(state): State<AppState>,
) -> Result<Json<Vec<TaskSummaryResponse>>, AppError> {
    let tasks = state.service.list_tasks().await?;

    let response: Vec<TaskSummaryResponse> = tasks
        .into_iter()
        .map(|task| TaskSummaryResponse {
            id: task.id,
            name: task.name,
            status: if task.deleted_at.is_some() {
                "deleted".to_string()
            } else {
                "active".to_string()
            },
            deleted_at: task.deleted_at,
        })
        .collect();

    Ok(Json(response))
}
