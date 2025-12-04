use reqwest::Client;
use serde_json::{Value, json};
use sqlx::SqlitePool;
use tokio::{net::TcpListener, sync::mpsc};
use tokio_util::sync::CancellationToken;

use crate::{api, scheduler, service::TaskService};

async fn spawn_app(pool: SqlitePool) -> String {
    let (scheduler_tx, scheduler_rx) = mpsc::channel::<()>(100);
    let token = CancellationToken::new();

    let service = TaskService::new(pool.clone(), scheduler_tx);
    let scheduler_service = service.clone();

    tokio::spawn(async move {
        scheduler::run_scheduler(scheduler_service, scheduler_rx, token).await;
    });

    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("Failed to bind address");

    listener
        .set_nonblocking(true)
        .expect("Failed to set non-blocking");
    let port = listener.local_addr().unwrap().port();
    let address = format!("http://127.0.0.1:{}", port);

    let app = api::router(service);

    tokio::spawn(async move {
        axum::serve(TcpListener::from_std(listener).unwrap(), app)
            .await
            .unwrap();
    });

    address
}

#[sqlx::test]
async fn test_e2e_execution(pool: SqlitePool) {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("info,task_scheduler=debug,sqlx=error")
        .try_init();

    let address = spawn_app(pool.clone()).await;
    let client = Client::new();

    let target_url = format!("{}/tasks", address);

    let response = client
        .post(format!("{}/tasks", &address))
        .json(&json!({
            "name": "e2e_test_task",
            "task_type": "once",
            "trigger_at": chrono::Utc::now().to_rfc3339(),
            "payload": { "url": target_url, "method": "GET" }
        }))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status().as_u16(), 200);
    let body: Value = response.json().await.unwrap();
    let task_id = body["id"].as_str().unwrap();

    // Parse task_id as UUID object as it is stored as BLOB in the DB
    let task_uuid = uuid::Uuid::parse_str(task_id).expect("Invalid UUID format");

    // Wait for some time to allow the scheduler to process the task
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM executions WHERE task_id = ?")
        .bind(task_uuid)
        .fetch_one(&pool)
        .await
        .expect("Failed to query DB");
    assert_eq!(
        count, 1,
        "There should be one execution record for the task"
    );

    let _ = std::fs::remove_file("e2e_test.db");
    let _ = std::fs::remove_file("e2e_test.db-shm");
    let _ = std::fs::remove_file("e2e_test.db-wal");
}

#[sqlx::test]
async fn test_scheduler_handles_http_failure(pool: SqlitePool) {
    let address = spawn_app(pool.clone()).await;
    let client = Client::new();

    let response = client
        .post(format!("{}/tasks", &address))
        .json(&json!({
            "name": "e2e_failure_task",
            "task_type": "once",
            "trigger_at": chrono::Utc::now().to_rfc3339(),
            "payload": { "url": "127.0.0.1:9999", "method": "GET" } // Invalid URL to trigger failure
        }))
        .send()
        .await
        .expect("Failed to send request");

    let task_id = response.json::<Value>().await.unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();

    tokio::time::sleep(std::time::Duration::from_secs(3)).await;

    let status: String = sqlx::query_scalar("SELECT status FROM executions WHERE task_id = ?")
        .bind(uuid::Uuid::parse_str(&task_id).unwrap())
        .fetch_one(&pool)
        .await
        .expect("Execution log missing");

    assert_eq!(status, "failure");

    let _ = std::fs::remove_file("e2e_test.db");
    let _ = std::fs::remove_file("e2e_test.db-shm");
    let _ = std::fs::remove_file("e2e_test.db-wal");
}
