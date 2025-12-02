pub mod dto;

use crate::api::dto::CreateTaskReq;
use crate::errors::AppError;
use crate::service::TaskService;
use axum::{
    Json, Router,
    extract::{Request, State},
    http::HeaderValue,
    routing::post,
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

pub fn router(service: TaskService) -> Router {
    let state = AppState { service };

    let x_request_id = "x-request-id".parse::<axum::http::HeaderName>().unwrap();
    Router::new()
        .route("/tasks", post(create_task))
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

async fn create_task(
    State(state): State<AppState>,
    Json(payload): Json<CreateTaskReq>,
) -> Result<Json<Value>, AppError> {
    let task_id = state.service.create_task(payload).await?;

    Ok(Json(json!({ "status": "created","id": task_id })))
}
