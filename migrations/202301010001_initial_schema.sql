CREATE TABLE tasks (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    task_type TEXT NOT NULL,
    trigger_at DATETIME NOT NULL,
    interval_seconds INTEGER,
    payload TEXT NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    deleted_at DATETIME
);

CREATE INDEX idx_tasks_trigger_at ON tasks(trigger_at);

CREATE TABLE executions (
    id TEXT PRIMARY KEY NOT NULL,
    task_id TEXT NOT NULL,
    executed_at DATETIME NOT NULL,
    output TEXT NOT NULL,
    status TEXT NOT NULL,
    FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE
);
