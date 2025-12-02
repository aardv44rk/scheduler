use std::time::Duration;

use crate::{db::queries::TaskRepository, service::TaskService};
use chrono::Utc;
use tokio::sync::mpsc;

pub async fn run_scheduler(service: TaskService, mut rx: mpsc::Receiver<()>) {
    let repo = TaskRepository::new(&service.get_pool());

    loop {
        let next_task = match repo.get_next_pending_task().await {
            Ok(task) => task,
            Err(e) => {
                tracing::error!("Failed to fetch next task: {:?}", e);
                tokio::time::sleep(Duration::from_secs(5)).await;
                continue;
            }
        };

        let sleep_duration = if let Some(ref task) = next_task {
            let now = Utc::now();

            if task.trigger_at <= now {
                Duration::ZERO
            } else {
                (task.trigger_at - now).to_std().unwrap_or(Duration::ZERO)
            }
        } else {
            Duration::from_secs(3600)
        };

        tracing::info!(
            "Scheduler sleeping for {:?}. Next task: {:?}",
            sleep_duration,
            next_task.as_ref().map(|t| &t.name)
        );

        tokio::select! {
            _ = tokio::time::sleep(sleep_duration) => {
                if let Some(task) = next_task {
                    if task.trigger_at <= Utc::now() {
                        if let Err(e) = service.process_task(task).await {
                        tracing::error!("Error processing task: {:?}", e);
                        }
                    }
                }
            }
            _ = rx.recv() => {
                tracing::info!("Received new task notification.");
            }
        }
    }
}
