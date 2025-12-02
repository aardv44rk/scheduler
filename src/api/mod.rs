pub mod dto;

use crate::api::dto::CreateTaskReq;
use crate::errors::AppError;
use crate::service::TaskService;
use axum::{Json, Router, extract::State, http::HeaderValue, routing::post};
use serde_json::{Value, json};
use tower_http::{
    request_id::{MakeRequestId, RequestId, SetRequestIdLayer},
    trace::{DefaultMakeSpan, DefaultOnRequest, DefaultOnResponse, TraceLayer},
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

        Some(RequestId::from(header_value))
    }
}

pub fn router(service: TaskService) -> Router {
    let state = AppState { service };

    Router::new()
        .route("/tasks", post(create_task))
        .with_state(state)
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().include_headers(false))
                .on_request(DefaultOnRequest::new().level(tracing::Level::INFO))
                .on_response(DefaultOnResponse::new().level(tracing::Level::INFO)),
        )
        .layer(SetRequestIdLayer::new(
            "x-request-id".parse().unwrap(),
            MakeUuidRequest,
        ))
}

async fn create_task(
    State(state): State<AppState>,
    Json(payload): Json<CreateTaskReq>,
) -> Result<Json<Value>, AppError> {
    let task_id = state.service.create_task(payload).await?;

    Ok(Json(json!({ "status": "created","id": task_id })))
}
