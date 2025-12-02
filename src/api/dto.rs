use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Deserialize)]
pub struct CreateTaskReq {
    pub name: String,
    pub task_type: String,
    pub trigger_at: DateTime<Utc>,
    pub interval_seconds: Option<i64>,
    pub payload: Option<Value>,
}

#[derive(Serialize)]
pub struct TaskResponse {
    pub id: String,
    pub name: String,
    pub task_type: String,
    pub trigger_at: DateTime<Utc>,
    pub interval_seconds: Option<i64>,
    pub payload: Value,
}
