use crate::domain::Task;
use chrono::Utc;
use serde_json::Value;
use sqlx::{Executor, Row, Sqlite, SqlitePool, types::Json};
use uuid::Uuid;

pub struct TaskRepository<'a> {
    pub pool: &'a SqlitePool,
}

impl<'a> TaskRepository<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    /// Creates a new task in the database.
    ///
    /// # Arguments
    ///
    /// * `task` - A reference to the Task entity to be created.
    ///
    /// # Returns
    /// * `sqlx::Result<()>` - Result indicating success or failure of the operation.
    pub async fn create_task(&self, task: &Task) -> sqlx::Result<()> {
        sqlx::query(
            r#"
            INSERT INTO tasks (id, name, task_type, trigger_at, interval_seconds, payload)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(task.id)
        .bind(&task.name)
        .bind(task.task_type.clone())
        .bind(task.trigger_at)
        .bind(task.interval_seconds)
        .bind(Json(&task.payload))
        .execute(self.pool)
        .await?;

        Ok(())
    }

    /// Retrieves a task by its ID from the database.
    ///
    /// # Arguments
    ///
    /// * `id` - The UUID of the task to retrieve.
    ///
    /// # Returns
    /// * `sqlx::Result<Option<Task>>` - Result containing the Task if found, or None if not found.
    pub async fn get_task(&self, id: Uuid) -> sqlx::Result<Option<Task>> {
        let row = sqlx::query(
            r#"
            SELECT id, name, task_type, trigger_at, interval_seconds, payload, deleted_at
            FROM tasks
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(self.pool)
        .await?;

        let row = match row {
            Some(row) => row,
            None => return Ok(None),
        };
        Ok(Some(Task {
            id: row.try_get("id")?,
            name: row.try_get("name")?,
            task_type: row.try_get("task_type")?,
            trigger_at: row.try_get("trigger_at")?,
            interval_seconds: row.try_get("interval_seconds")?,
            payload: row.try_get::<Json<Value>, _>("payload")?.0,
            deleted_at: row.try_get("deleted_at")?,
        }))
    }

    pub async fn delete_task(&self, id: Uuid) -> sqlx::Result<u64> {
        Self::delete_task_with_executor(self.pool, id).await
    }

    /// Soft deletes a task by setting its deleted_at timestamp.
    ///
    /// # Arguments
    ///
    /// * `executor` - An executor that can execute the query (e.g., a connection or transaction).
    /// * `id` - The UUID of the task to soft delete.
    ///
    /// # Returns
    /// * `sqlx::Result<u64>` - Result containing the number of rows affected.
    pub async fn delete_task_with_executor<'c, E>(executor: E, id: Uuid) -> sqlx::Result<u64>
    where
        E: Executor<'c, Database = Sqlite>,
    {
        tracing::info!("DEBUG: Running Soft Delete for Task {}", id);
        let result = sqlx::query("UPDATE tasks SET deleted_at = ? WHERE id = ?")
            .bind(Utc::now())
            .bind(id)
            .execute(executor)
            .await?;

        Ok(result.rows_affected())
    }

    pub async fn update_trigger_with_executor<'c, E>(
        executor: E,
        id: Uuid,
        new_trigger_at: chrono::DateTime<Utc>,
    ) -> sqlx::Result<u64>
    where
        E: Executor<'c, Database = Sqlite>,
    {
        let result = sqlx::query(
            r#"
            UPDATE tasks
            SET trigger_at = ?
            WHERE id = ?
            "#,
        )
        .bind(new_trigger_at)
        .bind(id)
        .execute(executor)
        .await?;

        Ok(result.rows_affected())
    }

    pub async fn get_next_pending_task(&self) -> sqlx::Result<Option<Task>> {
        let row = sqlx::query(
            r#"
            SELECT id, name, task_type, trigger_at, interval_seconds, payload, deleted_at
            FROM tasks
            WHERE deleted_at IS NULL
            ORDER BY trigger_at ASC
            LIMIT 1
            "#,
        )
        .fetch_optional(self.pool)
        .await?;

        let row = match row {
            Some(row) => row,
            None => return Ok(None),
        };

        Ok(Some(Task {
            id: row.try_get("id")?,
            name: row.try_get("name")?,
            task_type: row.try_get("task_type")?,
            trigger_at: row.try_get("trigger_at")?,
            interval_seconds: row.try_get("interval_seconds")?,
            payload: row.try_get::<Json<Value>, _>("payload")?.0,
            deleted_at: row.try_get("deleted_at")?,
        }))
    }

    pub async fn get_all_tasks(&self) -> sqlx::Result<Vec<Task>> {
        sqlx::query_as::<_, Task>(
            r#"
            SELECT id, name, task_type, trigger_at, interval_seconds, payload, deleted_at
            FROM tasks
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(self.pool)
        .await
    }
}
