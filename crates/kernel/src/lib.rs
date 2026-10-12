use async_trait::async_trait;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Book {
    pub id: i64,
    pub title: String,
}

#[derive(Debug, Error)]
#[error("repository operation failed")]
pub struct RepositoryError(#[source] pub Box<dyn std::error::Error + Send + Sync>);

#[async_trait]
pub trait BookRepository: Send + Sync {
    async fn find(&self, id: i64) -> Result<Option<Book>, RepositoryError>;
}
