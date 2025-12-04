use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use sqlx::{Pool, Sqlite};
use std::str::FromStr;
use std::time::Duration;

pub mod queries;

#[cfg(test)]
mod tests;

/// Initialize the SQLite connection pool with appropriate options.
///
/// # Arguments
///
/// * `database_url` - The database URL string.
///
/// # Returns
/// * `Pool<Sqlite>` - The initialized SQLite connection pool.
pub async fn init_pool(database_url: &str) -> Pool<Sqlite> {
    let mut options = SqliteConnectOptions::from_str(database_url)
        .expect("Invalid DATABASE_URL")
        .journal_mode(SqliteJournalMode::Wal)
        .foreign_keys(true)
        .create_if_missing(true);

    options = options.busy_timeout(Duration::from_secs(5));

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await
        .expect("Failed to create db pool");

    sqlx::query("PRAGMA foreign_keys = ON")
        .execute(&pool)
        .await
        .expect("Failed to enable foreign keys");

    pool
}
