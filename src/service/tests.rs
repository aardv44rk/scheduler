use crate::{domain::Task, service::TaskService};
use chrono::Duration;
use chrono::Utc;
use serde_json::json;
use sqlx::SqlitePool;
use tokio::sync::mpsc;

fn setup_service(pool: SqlitePool) -> TaskService {
    let (tx, _) = mpsc::channel(1);
    TaskService::new(pool, tx)
}

#[sqlx::test]
async fn test_process_task_reschedules(pool: SqlitePool) -> sqlx::Result<()> {
    let service = setup_service(pool.clone());
    let repo = crate::db::queries::TaskRepository::new(&pool);

    // Create an interval task
    let trigger_at = chrono::Utc::now() - chrono::Duration::minutes(1); // in the past to trigger immediately
    let interval_seconds = 60;
    let payload = json!({ "url": "http://example.com" }); // dummy payload

    let task = Task::new_interval("interval_task", trigger_at, interval_seconds, payload);

    repo.create_task(&task).await?;

    // Process the task
    service
        .process_task(task.clone())
        .await
        .expect("Process task failed");

    // Fetch the task again to verify it was rescheduled
    let updated_task = repo.get_task(task.id).await?.expect("Task should exist");

    let expected_trigger = Utc::now() + Duration::seconds(interval_seconds);

    let diff = updated_task
        .trigger_at
        .signed_duration_since(expected_trigger)
        .num_milliseconds()
        .abs();

    assert!(
        diff < 100,
        "Task should have incremented by interval relevant to now"
    );

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM executions WHERE task_id = ?")
        .bind(task.id)
        .fetch_one(&pool)
        .await?;

    assert_eq!(count, 1, "There should be one execution record");

    Ok(())
}

#[sqlx::test]
async fn test_process_task_once_deletes(pool: SqlitePool) -> sqlx::Result<()> {
    let service = setup_service(pool.clone());
    let repo = crate::db::queries::TaskRepository::new(&pool);

    // Create a once task
    let task = Task::new_once("once_task", Utc::now(), json!({}));
    repo.create_task(&task).await?;

    service
        .process_task(task.clone())
        .await
        .expect("Process task failed");

    let fetched_task = repo
        .get_task(task.id)
        .await?
        .expect("Task should exist even if soft deleted");

    assert!(
        fetched_task.deleted_at.is_some(),
        "Task should be marked as deleted after execution"
    );

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM executions WHERE task_id = ?")
        .bind(task.id)
        .fetch_one(&pool)
        .await?;
    assert_eq!(count, 1, "There should be one execution record");

    Ok(())
}

#[sqlx::test]
async fn test_interval_calculates_next_trigger_correctly(pool: SqlitePool) -> sqlx::Result<()> {
    let repo = crate::db::queries::TaskRepository::new(&pool);
    let service = setup_service(pool.clone());

    let task = Task::new_interval("test", Utc::now(), 3600, json!({}));
    repo.create_task(&task).await?;

    service.process_task(task.clone()).await.unwrap();

    let updated_task = repo.get_task(task.id).await?.unwrap();

    let expected = Utc::now() + Duration::seconds(3600);
    let diff = updated_task
        .trigger_at
        .signed_duration_since(expected)
        .num_seconds()
        .abs();

    assert!(
        diff < 5,
        "Next trigger should be approximately one hour later! Got {}, expected {}",
        updated_task.trigger_at,
        expected
    );

    Ok(())
}
