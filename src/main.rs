use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use std::str::FromStr;
use std::time::Duration;
use tokio::{net::TcpListener, sync::mpsc};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use task_scheduler::{api, config::Config, service::TaskService};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::from_env()?;

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(&config.rust_log))
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting Task Scheduler...");

    let connection_options = SqliteConnectOptions::from_str(&config.db_url)?
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .foreign_keys(true)
        .busy_timeout(Duration::from_secs(30));

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(connection_options)
        .await?;

    tracing::info!("Database connection pool established.");

    sqlx::migrate!("./migrations").run(&pool).await?;
    tracing::info!("Migrations applied successfully.");

    let (scheduler_tx, scheduler_rx) = mpsc::channel::<()>(100);

    tracing::info!("Created scheduler channels.");

    let service = TaskService::new(pool.clone(), scheduler_tx);

    let scheduler_service = service.clone();

    tokio::spawn(async move {
        tracing::info!("Scheduler background task started.");
        task_scheduler::scheduler::run_scheduler(scheduler_service, scheduler_rx).await;
    });
    tracing::info!("Task service initialized.");

    let app = api::router(service);

    let addr = format!("0.0.0.0:{}", config.server_port);
    tracing::info!("API Server listening on on {}", addr);
    let listener = TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
