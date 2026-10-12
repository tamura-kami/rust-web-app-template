use async_trait::async_trait;
use kernel::{Book, BookRepository, RepositoryError};
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::time::Duration;

pub async fn connect(database_url: &str) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(5))
        .connect(database_url)
        .await
}

pub async fn migrate(pool: &PgPool) -> Result<(), sqlx::migrate::MigrateError> {
    sqlx::migrate!("../../migrations").run(pool).await
}

pub struct PgBookRepository {
    pool: PgPool,
}

impl PgBookRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl BookRepository for PgBookRepository {
    async fn find(&self, id: i64) -> Result<Option<Book>, RepositoryError> {
        let row: Option<(i64, String)> =
            sqlx::query_as("SELECT id, title FROM books WHERE id = $1")
                .bind(id)
                .fetch_optional(&self.pool)
                .await
                .map_err(|error| RepositoryError(Box::new(error)))?;
        Ok(row.map(|(id, title)| Book { id, title }))
    }
}
