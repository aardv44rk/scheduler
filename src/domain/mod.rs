use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{FromRow, Type};
use uuid::Uuid;

// Enums

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[sqlx(rename_all = "lowercase")]
pub enum TaskType {
    Once,
    Interval,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[sqlx(rename_all = "lowercase")]
pub enum ExecutionStatus {
    Success,
    Failure,
}

// Structs
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Execution {
    pub id: Uuid,
    pub task_id: Uuid,
    pub executed_at: DateTime<Utc>,
    pub output: Value,
    pub status: ExecutionStatus,
}

impl Execution {
    pub fn new(task_id: Uuid, output: Value, status: ExecutionStatus) -> Self {
        Execution {
            id: Uuid::new_v4(),
            task_id,
            executed_at: Utc::now(),
            output,
            status,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, PartialEq)]
pub struct Task {
    pub id: Uuid,
    pub name: String,
    pub task_type: TaskType,
    pub trigger_at: DateTime<Utc>,
    pub interval_seconds: Option<i64>,
    pub payload: Value,
}

impl Task {
    pub fn new_once(name: impl Into<String>, trigger_at: DateTime<Utc>, payload: Value) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            task_type: TaskType::Once,
            trigger_at,
            interval_seconds: None,
            payload,
        }
    }

    pub fn new_interval(
        name: impl Into<String>,
        trigger_at: DateTime<Utc>,
        interval_seconds: i64,
        payload: Value,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            task_type: TaskType::Interval,
            trigger_at,
            interval_seconds: Some(interval_seconds),
            payload,
        }
    }
}
