use adapter::{PgBookRepository, connect, migrate};
use kernel::BookRepository;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub books: Arc<dyn BookRepository>,
}

impl AppState {
    pub async fn build(database_url: &str) -> anyhow::Result<Self> {
        let pool = connect(database_url).await?;
        migrate(&pool).await?;
        Ok(Self {
            books: Arc::new(PgBookRepository::new(pool)),
        })
    }
}
