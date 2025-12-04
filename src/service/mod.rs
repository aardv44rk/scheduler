use crate::api::dto::CreateTaskReq;
use crate::db::queries::TaskRepository;
use crate::domain::{Execution, ExecutionStatus, Task, TaskType};
use crate::errors::AppError;
use serde_json::json;
use sqlx::{SqlitePool, types::Json};
use tokio::sync::mpsc::Sender;
use uuid::Uuid;

#[cfg(test)]
mod tests;

#[derive(Clone)]
pub struct TaskService {
    db_pool: SqlitePool,
    scheduler_tx: Sender<()>,
}

impl TaskService {
    pub fn new(db_pool: SqlitePool, scheduler_tx: Sender<()>) -> Self {
        Self {
            db_pool,
            scheduler_tx,
        }
    }

    pub fn get_pool(&self) -> &SqlitePool {
        &self.db_pool
    }

    pub async fn delete_task(&self, id: Uuid) -> Result<(), AppError> {
        let repo = TaskRepository::new(&self.db_pool);

        let rows_affected = repo.delete_task(id).await?;
        if rows_affected == 0 {
            return Err(AppError::NotFound);
        }

        Ok(())
    }

    /// Creates a new task based on the provided request data.
    ///
    /// # Arguments
    ///
    /// * `req` - A 'CreateTaskReq' containing task details.
    ///
    /// # Errors
    ///
    /// * Returns 'AppError::ValidationError' if:
    /// * 'task_type' is invalid.
    /// * 'Interval' task is missing 'interval_seconds'
    /// * 'Interval' task has 'interval_seconds' less than 1.
    ///
    /// * Returns AppError::Database if insert fails.
    pub async fn create_task(&self, req: CreateTaskReq) -> Result<Uuid, AppError> {
        let task_type = match req.task_type.as_str() {
            "once" => TaskType::Once,
            "interval" => TaskType::Interval,
            _ => {
                return Err(AppError::ValidationError(
                    "Invalid task_type. Use 'once' or 'interval'".into(),
                ));
            }
        };

        if task_type == TaskType::Interval {
            match req.interval_seconds {
                Some(seconds) if seconds < 1 => {
                    // limit to at least 1 second to avoid loops
                    return Err(AppError::ValidationError(
                        "interval_seconds must be at least 1 second".into(),
                    ));
                }
                None => {
                    return Err(AppError::ValidationError(
                        "interval_seconds is required for interval tasks".into(),
                    ));
                }
                _ => {} // valid
            }
        }

        // Map DTO to Domain Entity
        let payload = req.payload.unwrap_or(json!({}));

        let task = match task_type {
            TaskType::Once => Task::new_once(req.name, req.trigger_at, payload),
            TaskType::Interval => Task::new_interval(
                req.name,
                req.trigger_at,
                req.interval_seconds.unwrap(),
                payload,
            ),
        };

        // Save to DB
        let repo = TaskRepository::new(&self.db_pool);
        repo.create_task(&task).await?;

        // Notify scheduler
        let _ = self.scheduler_tx.try_send(());

        Ok(task.id)
    }

    /// Processes a task: executes its logic, records execution, and updates/deletes the task as needed.
    ///
    /// # Arguments
    ///
    /// * `task` - The Task to be processed.
    ///
    /// # Errors
    ///
    /// * Returns 'AppError::Database' for any database operation failures.
    ///
    /// Returns 'Ok(())' even if the task was deleted during processing.
    pub async fn process_task(&self, task: Task) -> Result<(), AppError> {
        tracing::info!(
            task_id = %task.id,
            name = %task.name,
            "Processing Task"
        );

        let (output, status) = match self.execute_logic(&task).await {
            Ok(val) => (val, ExecutionStatus::Success),
            Err(e) => (json!({ "error": e.to_string() }), ExecutionStatus::Failure),
        };

        let mut scheduler_tx = self.db_pool.begin().await?;

        let exec = Execution::new(task.id, output, status);

        let id = exec.id;
        let task_id = exec.task_id;
        let executed_at = exec.executed_at;
        let output = Json(&exec.output);
        let exec_status = exec.status;

        let db_result = sqlx::query(
            r#"
            INSERT INTO executions (id, task_id, executed_at, output, status)
            VALUES (?1, ?2, ?3, ?4, ?5)
            "#,
        )
        .bind(id)
        .bind(task_id)
        .bind(executed_at)
        .bind(output)
        .bind(exec_status)
        .execute(&mut *scheduler_tx)
        .await;

        match db_result {
            Ok(_) => match task.task_type {
                // For once tasks, delete after execution
                TaskType::Once => {
                    TaskRepository::delete_task_with_executor(&mut *scheduler_tx, task.id).await?;
                }
                // For interval tasks, calculate and update next trigger time
                TaskType::Interval => {
                    if let Some(seconds) = task.interval_seconds {
                        let next_trigger = chrono::Utc::now() + chrono::Duration::seconds(seconds);

                        TaskRepository::update_trigger_with_executor(
                            &mut *scheduler_tx,
                            task.id,
                            next_trigger,
                        )
                        .await?;
                    }
                }
            },
            // Catch foreign key violation if task was deleted during processing here
            //
            Err(sqlx::Error::Database(e)) if e.is_foreign_key_violation() => {
                tracing::warn!("Task {} was deleted during execution.", task.id);
                scheduler_tx.rollback().await?;
                return Ok(());
            }

            Err(e) => return Err(AppError::Database(e)),
        }

        scheduler_tx.commit().await?;
        tracing::info!("Task processed succesfully!");

        Ok(())
    }

    /// Executes the HTTP webhook defined in the task payload.
    ///
    /// # Arguments
    ///
    /// * `task` - The Task containing the webhook details.
    ///
    /// # Errors
    ///
    /// * Returns an error string if the HTTP request fails or if required fields are missing.
    ///
    /// Returns the HTTP response as JSON on success.
    async fn execute_webhook(&self, task: &Task) -> Result<serde_json::Value, String> {
        let url = task
            .payload
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'url' in payload")?;

        let method = task
            .payload
            .get("method")
            .and_then(|v| v.as_str())
            .unwrap_or("GET");

        let value = json!({});
        let body = task.payload.get("body").unwrap_or(&value);

        let client = reqwest::Client::new();

        let builder = match method {
            "POST" => client.post(url).json(body),
            "PUT" => client.put(url).json(body),
            "DELETE" => client.delete(url),
            _ => client.get(url),
        };

        let response = builder
            .send()
            .await
            .map_err(|e| format!("HTTP request failed: {}", e))?;

        let status = response.status();
        let text = response.text().await.unwrap_or_default();

        if status.is_success() {
            Ok(json!({ "status": status.as_u16(), "response": text }))
        } else {
            Err(format!("HTTP Error {}: {}", status.as_u16(), text))
        }
    }
}
