use crate::api::router;
use crate::service::TaskService;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{Value, from_slice, json};
use sqlx::SqlitePool;
use tokio::sync::mpsc;
use tower::util::ServiceExt;

#[sqlx::test]
async fn test_create_task_success(pool: SqlitePool) -> sqlx::Result<()> {
    let (tx, _rx) = mpsc::channel(1);
    let service = TaskService::new(pool.clone(), tx);
    let app = router(service);

    // create request
    let payload = json!({
        "name": "test_task",
        "task_type": "once",
        "trigger_at": chrono::Utc::now().to_rfc3339(),
        "payload": { "key": "value" }
    });

    let req = Request::builder()
        .method("POST")
        .uri("/tasks")
        .header("Content-Type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();

    let response = app.oneshot(req).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body_json: Value = from_slice(&body_bytes).unwrap();

    assert_eq!(body_json["status"], "created");
    assert!(body_json["id"].as_str().is_some());

    Ok(())
}
#[sqlx::test]
async fn test_create_task_validation_error(pool: SqlitePool) -> sqlx::Result<()> {
    let (tx, _rx) = mpsc::channel(1);
    let service = TaskService::new(pool.clone(), tx);
    let app = router(service);

    // create request
    let payload = json!({
        "name": "invalid_task",
        "task_type": "interval",
        "trigger_at": chrono::Utc::now().to_rfc3339(),
        //missing interval seconds
    });

    let req = Request::builder()
        .method("POST")
        .uri("/tasks")
        .header("Content-Type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();

    let response = app.oneshot(req).await.unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    Ok(())
}
