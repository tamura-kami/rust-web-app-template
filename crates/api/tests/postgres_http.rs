use axum::{
    body::{Body, to_bytes},
    http::Request,
};
use registry::AppState;
use tower::ServiceExt;

#[tokio::test]
#[ignore = "requires migrated PostgreSQL; run with DATABASE_URL and --ignored"]
async fn http_reads_postgres() {
    let state = AppState::build(&std::env::var("DATABASE_URL").unwrap())
        .await
        .unwrap();
    let response = api::router(state, &[])
        .unwrap()
        .oneshot(
            Request::builder()
                .uri("/books/1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let body = to_bytes(response.into_body(), 1024).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["title"], "Rust Web API sample");
}
