use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

/// Request DTO for creating a new task.
#[derive(Deserialize)]
pub struct CreateTaskReq {
    pub name: String,
    pub task_type: String,
    pub trigger_at: DateTime<Utc>,
    pub interval_seconds: Option<i64>,
    pub payload: Option<Value>,
}

/// Response DTO for returning task details.
#[derive(Serialize)]
pub struct TaskResponse {
    pub id: String,
    pub name: String,
    pub task_type: String,
    pub trigger_at: DateTime<Utc>,
    pub interval_seconds: Option<i64>,
    pub payload: Value,
}

/// Response DTO for returning a summary of a task.
#[derive(Serialize)]
pub struct TaskSummaryResponse {
    pub id: Uuid,
    pub name: String,
    pub status: String,
    pub deleted_at: Option<DateTime<Utc>>,
}
