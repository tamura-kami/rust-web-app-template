use axum::{
    Json, Router,
    extract::{Path, State},
    http::{HeaderValue, Method, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
};
use registry::AppState;
use serde::Serialize;
use tower_http::{
    cors::{AllowOrigin, CorsLayer},
    trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer},
};

pub fn router(
    state: AppState,
    origins: &[String],
) -> Result<Router, axum::http::header::InvalidHeaderValue> {
    let origins = origins
        .iter()
        .map(|origin| origin.parse::<HeaderValue>())
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Router::new()
        .route(
            "/health",
            get(|| async { Json(serde_json::json!({"status": "ok"})) }),
        )
        .route("/books/{id}", get(find_book))
        .fallback(|| async { ApiError::NotFound })
        .with_state(state)
        .layer(
            CorsLayer::new()
                .allow_origin(AllowOrigin::list(origins))
                .allow_methods([Method::GET]),
        )
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().level(tracing::Level::INFO))
                .on_response(DefaultOnResponse::new().level(tracing::Level::INFO)),
        ))
}

#[derive(Serialize)]
struct BookResponse {
    id: i64,
    title: String,
}

async fn find_book(
    State(state): State<AppState>,
    id: Result<Path<i64>, axum::extract::rejection::PathRejection>,
) -> Result<Json<BookResponse>, ApiError> {
    let Path(id) = id.map_err(|_| ApiError::BadRequest)?;
    if id <= 0 {
        return Err(ApiError::BadRequest);
    }
    let book = state
        .books
        .find(id)
        .await
        .map_err(|error| {
            tracing::error!(error = ?error, "book lookup failed");
            ApiError::Internal
        })?
        .ok_or(ApiError::NotFound)?;
    Ok(Json(BookResponse {
        id: book.id,
        title: book.title,
    }))
}

enum ApiError {
    BadRequest,
    NotFound,
    Internal,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code) = match self {
            Self::BadRequest => (StatusCode::BAD_REQUEST, "invalid_book_id"),
            Self::NotFound => (StatusCode::NOT_FOUND, "not_found"),
            Self::Internal => (StatusCode::INTERNAL_SERVER_ERROR, "internal_error"),
        };
        (status, Json(serde_json::json!({"error": code}))).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use axum::{
        body::{Body, to_bytes},
        http::Request,
    };
    use kernel::{Book, BookRepository, RepositoryError};
    use std::sync::Arc;
    use tower::ServiceExt;

    struct FakeRepository {
        fail: bool,
    }
    #[async_trait]
    impl BookRepository for FakeRepository {
        async fn find(&self, id: i64) -> Result<Option<Book>, RepositoryError> {
            if self.fail {
                return Err(RepositoryError(Box::new(std::io::Error::other(
                    "secret database details",
                ))));
            }
            Ok((id == 1).then(|| Book {
                id,
                title: "Sample".into(),
            }))
        }
    }
    fn app(fail: bool) -> Router {
        router(
            AppState {
                books: Arc::new(FakeRepository { fail }),
            },
            &["https://example.com".into()],
        )
        .unwrap()
    }
    #[tokio::test]
    async fn responses_and_validation() {
        for (path, status, body) in [
            ("/health", 200, serde_json::json!({"status":"ok"})),
            (
                "/books/1",
                200,
                serde_json::json!({"id":1,"title":"Sample"}),
            ),
            ("/books/2", 404, serde_json::json!({"error":"not_found"})),
            (
                "/books/0",
                400,
                serde_json::json!({"error":"invalid_book_id"}),
            ),
            (
                "/books/abc",
                400,
                serde_json::json!({"error":"invalid_book_id"}),
            ),
            ("/missing", 404, serde_json::json!({"error":"not_found"})),
        ] {
            let response = app(false)
                .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(response.status().as_u16(), status, "{path}");
            let bytes = to_bytes(response.into_body(), 1024).await.unwrap();
            assert_eq!(
                serde_json::from_slice::<serde_json::Value>(&bytes).unwrap(),
                body
            );
        }
    }
    #[tokio::test]
    async fn internal_error_is_sanitized() {
        let response = app(true)
            .oneshot(
                Request::builder()
                    .uri("/books/1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        let bytes = to_bytes(response.into_body(), 1024).await.unwrap();
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&bytes).unwrap(),
            serde_json::json!({"error":"internal_error"})
        );
    }
    #[tokio::test]
    async fn cors_only_allows_configured_origin() {
        for (origin, allowed) in [
            ("https://example.com", true),
            ("https://evil.example", false),
        ] {
            let response = app(false)
                .oneshot(
                    Request::builder()
                        .uri("/health")
                        .header("origin", origin)
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(
                response
                    .headers()
                    .get("access-control-allow-origin")
                    .is_some(),
                allowed
            );
        }
    }
}
