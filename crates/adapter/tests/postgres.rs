use adapter::{PgBookRepository, migrate};
use kernel::BookRepository;
use sqlx::PgPool;

#[sqlx::test]
#[ignore = "requires PostgreSQL; run with DATABASE_URL and --ignored"]
async fn migrations_and_repository(pool: PgPool) {
    migrate(&pool).await.unwrap();
    migrate(&pool).await.unwrap();
    let repository = PgBookRepository::new(pool);
    let book = repository.find(1).await.unwrap().unwrap();
    assert_eq!(book.title, "Rust Web API sample");
    assert!(repository.find(999).await.unwrap().is_none());
}
