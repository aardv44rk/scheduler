use crate::api::dto::CreateTaskReq;
use crate::db::queries::TaskRepository;
use crate::domain::{Execution, ExecutionStatus, Task, TaskType};
use crate::errors::AppError;
use serde_json::json;
use sqlx::{SqlitePool, types::Json};
use tokio::sync::mpsc::Sender;
use uuid::Uuid;

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

        if task_type == TaskType::Interval && req.interval_seconds.is_none() {
            return Err(AppError::ValidationError(
                "Interval tasks require interval_seconds".into(),
            ));
        }

        // 2. Map DTO to Domain Entity
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

        // 3. Save to DB
        let repo = TaskRepository::new(&self.db_pool);
        repo.create_task(&task).await?;

        // 4. Notify Scheduler (Fire and forget)
        // If the channel is full or receiver dropped, we log but don't fail the request
        let _ = self.scheduler_tx.try_send(());

        Ok(task.id)
    }

    pub async fn process_task(&self, task: Task) -> Result<(), AppError> {
        tracing::info!("Processing task: {}", task.name);

        let output = match self.execute_logic(&task).await {
            Ok(val) => val,
            Err(e) => json!({ "error": e.to_string() }),
        };

        let status = ExecutionStatus::Success;

        let mut scheduler_tx = self.db_pool.begin().await?;

        let exec = Execution::new(task.id, output, status);

        let id = exec.id;
        let task_id = exec.task_id;
        let executed_at = exec.executed_at;
        let output = Json(&exec.output);
        let exec_status = exec.status;

        sqlx::query(
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
        .await?;

        match task.task_type {
            TaskType::Once => {
                TaskRepository::delete_task_with_executor(&mut *scheduler_tx, task.id).await?;
            }
            TaskType::Interval => {
                // extract to helper function
                if let Some(seconds) = task.interval_seconds {
                    let next_trigger = task.trigger_at + chrono::Duration::seconds(seconds);

                    TaskRepository::update_trigger_with_executor(
                        &mut *scheduler_tx,
                        task.id,
                        next_trigger,
                    )
                    .await?;
                }
            }
        }

        scheduler_tx.commit().await?;
        tracing::info!("Task processed succesfully!");

        Ok(())
    }

    /// Dummy logic function
    async fn execute_logic(&self, task: &Task) -> Result<serde_json::Value, String> {
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        Ok(json!({ "result": "Task executed successfully", "payload_echo": task.payload }))
    }
}
