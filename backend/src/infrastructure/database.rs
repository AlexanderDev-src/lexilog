use std::path::Path;
use std::time::Duration;

use chrono::{DateTime, SecondsFormat, Utc};
use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};

use crate::domain::error::AppError;

/// Opens (or creates) the database and runs pending migrations.
///
/// The PRAGMAs are set here, on every connection, because some of them
/// (`foreign_keys`, `busy_timeout`) only last for one connection.
pub async fn connect(path: &str) -> Result<SqlitePool, Box<dyn std::error::Error>> {
    if let Some(dir) = Path::new(path).parent() {
        std::fs::create_dir_all(dir)?;
    }

    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true)
        // WAL: readers don't block the writer, and `.backup` works while the app runs.
        .journal_mode(SqliteJournalMode::Wal)
        // NORMAL is safe with WAL and much faster than FULL.
        .synchronous(SqliteSynchronous::Normal)
        .foreign_keys(true)
        // Wait up to 5 s for a lock instead of failing at once with "database is locked".
        .busy_timeout(Duration::from_secs(5));

    let pool = SqlitePoolOptions::new()
        .max_connections(4)
        .connect_with(options)
        .await?;

    // `migrate!` embeds the .sql files into the binary at compile time, and
    // `run` applies any that the database hasn't seen yet (tracked in `_sqlx_migrations`).
    sqlx::migrate!("./migrations").run(&pool).await?;
    Ok(pool)
}

/// The one timestamp format we store: `2026-09-26T03:15:00Z`.
/// Same length every time, so text comparison == time comparison.
pub fn timestamp(dt: DateTime<Utc>) -> String {
    dt.to_rfc3339_opts(SecondsFormat::Secs, true)
}

/// Lets `?` convert a database error into our `AppError` automatically.
impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        log::error!("database error: {err}");
        AppError::Internal(err.to_string())
    }
}
