use crate::db::queries::TaskRepository;
use crate::domain::{Task, TaskType};
use chrono::{Duration, Utc};
use serde_json::json;
use sqlx::SqlitePool;

#[sqlx::test]
async fn test_create_and_get_task(pool: SqlitePool) -> sqlx::Result<()> {
    let repo = TaskRepository::new(&pool);

    // data setup
    let payload = json!({ "target_url": "https://example.com", "retries": 3 });
    let trigger_at = Utc::now();

    let new_task = Task::new_once("test_task", trigger_at, payload);

    repo.create_task(&new_task).await?;

    let fetched_task = repo.get_task(new_task.id).await?;
    assert!(fetched_task.is_some());
    let fetched_task = fetched_task.unwrap();

    assert_eq!(fetched_task.id, new_task.id);
    assert_eq!(fetched_task.name, "test_task");
    assert_eq!(fetched_task.task_type, TaskType::Once);
    assert!(
        fetched_task
            .trigger_at
            .signed_duration_since(new_task.trigger_at)
            .num_milliseconds()
            .abs()
            < 1000,
        "Timestamps should match closely"
    );
    assert_eq!(fetched_task.payload, new_task.payload);

    let deleted_count = repo.delete_task(new_task.id).await?;
    assert_eq!(deleted_count, 1);

    let deleted_task = repo.get_task(new_task.id).await?;
    assert!(
        deleted_task.is_some(),
        "Soft deleted task should still be retrievable"
    );

    let deleted_task = deleted_task.unwrap();
    assert!(
        deleted_task.deleted_at.is_some(),
        "Deleted task should have deleted_at set"
    );

    Ok(())
}

#[sqlx::test]
async fn test_get_next_pending_task_logic(pool: SqlitePool) -> sqlx::Result<()> {
    let repo = TaskRepository::new(&pool);

    let now = Utc::now();
    let future_time = now + Duration::hours(1);
    let past_time_old = now - Duration::hours(2); // Older
    let past_time_recent = now - Duration::hours(1); // Newer

    let future_task = Task::new_once("future", future_time, json!({}));
    repo.create_task(&future_task).await?;

    let pending = repo.get_next_pending_task().await?;
    assert!(pending.is_none(), "Should not pick up future tasks");

    let past_recent = Task::new_once("past_recent", past_time_recent, json!({}));
    repo.create_task(&past_recent).await?;

    let past_old = Task::new_once("past_old", past_time_old, json!({}));
    repo.create_task(&past_old).await?;

    // Scheduler should pick oldest pending task!
    let pending = repo.get_next_pending_task().await?;
    assert!(pending.is_some());
    let pending = pending.unwrap();

    assert_eq!(
        pending.id, past_old.id,
        "Should pick the oldest pending task first"
    );

    Ok(())
}

#[sqlx::test]
async fn test_interval_persistence(pool: SqlitePool) -> sqlx::Result<()> {
    let repo = TaskRepository::new(&pool);

    let task = Task::new_interval(
        "recur_task",
        Utc::now(),
        60, // 60 seconds
        json!({"type": "heartbeat"}),
    );

    repo.create_task(&task).await?;

    let fetched = repo.get_task(task.id).await?.unwrap();

    assert_eq!(fetched.task_type, TaskType::Interval);
    assert_eq!(fetched.interval_seconds, Some(60));

    Ok(())
}
