use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{FromRow, Type};
use uuid::Uuid;

// Enums

/// Represents execution mode of a task.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[sqlx(rename_all = "lowercase")]
pub enum TaskType {
    /// Task that runs only once at a specified time.
    Once,
    /// Task that runs at regular intervals.
    Interval,
}

/// Represents the status of a task execution.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[sqlx(rename_all = "lowercase")]
pub enum ExecutionStatus {
    /// Execution completed successfully.
    Success,
    /// Execution failed.
    Failure,
}

// Structs
/// Represents a task execution record.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Execution {
    /// Unique UUID v4.
    pub id: Uuid,
    /// Associated task's UUID.
    pub task_id: Uuid,
    /// Timestamp of when the execution occurred.
    pub executed_at: DateTime<Utc>,
    /// Output produced by the execution.
    pub output: Value,
    /// Status of the execution.
    pub status: ExecutionStatus,
}
/// Represents a scheduled task.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, PartialEq)]
pub struct Task {
    /// Unique UUID v4.
    pub id: Uuid,
    /// Name of the task.
    pub name: String,
    /// Type of the task (once or interval).
    pub task_type: TaskType,
    /// Timestamp when the task is scheduled to trigger.
    pub trigger_at: DateTime<Utc>,
    /// Interval in seconds for interval tasks.
    pub interval_seconds: Option<i64>,
    /// Payload containing task-specific data.
    pub payload: Value,
    /// If set, indicates the task is deleted and execution is skipped.
    pub deleted_at: Option<DateTime<Utc>>,
}

// Implementations

impl Task {
    pub fn new_once(name: impl Into<String>, trigger_at: DateTime<Utc>, payload: Value) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            task_type: TaskType::Once,
            trigger_at,
            interval_seconds: None,
            payload,
            deleted_at: None,
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
            deleted_at: None,
        }
    }
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
