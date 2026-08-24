use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

/// Application-wide error type. Any `anyhow::Error` (and thus most `?`
/// conversions) becomes a 500 response with the details logged server-side.
pub struct AppError(pub anyhow::Error);

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        tracing::error!("internal error: {:?}", self.0);
        (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error").into_response()
    }
}

impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        AppError(err.into())
    }
}

pub type AppResult<T> = Result<T, AppError>;
