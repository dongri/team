use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};

/// Application-wide error type. Errors bubble up via `?` and are rendered
/// as a styled error page; details are logged server-side only.
pub enum AppError {
    /// The resource does not exist, or the caller must not learn that it
    /// does (e.g. another user's draft).
    NotFound,
    /// The caller is signed in but not allowed to perform the action.
    Forbidden,
    Internal(anyhow::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            AppError::NotFound => not_found_page(),
            AppError::Forbidden => forbidden_page(),
            AppError::Internal(err) => {
                // `query_one` on a missing row is how handlers hit nonexistent
                // resources (e.g. /post/show/999) — show a 404, not a 500.
                if err
                    .chain()
                    .any(|c| c.to_string().contains("unexpected number of rows"))
                {
                    tracing::warn!("not found: {:?}", err);
                    return not_found_page();
                }
                tracing::error!("internal error: {:?}", err);
                error_page(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "500",
                    "Something went wrong",
                    "An unexpected error occurred. Please try again later.",
                )
            }
        }
    }
}

impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        AppError::Internal(err.into())
    }
}

pub type AppResult<T> = Result<T, AppError>;

/// True when the error chain contains a PostgreSQL unique-constraint
/// violation (e.g. duplicate username).
pub fn is_unique_violation(e: &anyhow::Error) -> bool {
    e.chain().any(|c| {
        c.downcast_ref::<tokio_postgres::Error>()
            .and_then(|pe| pe.code())
            .map(|code| *code == tokio_postgres::error::SqlState::UNIQUE_VIOLATION)
            .unwrap_or(false)
    })
}

pub fn not_found_page() -> Response {
    error_page(
        StatusCode::NOT_FOUND,
        "404",
        "Page not found",
        "The page you are looking for doesn't exist or may have been deleted.",
    )
}

pub fn forbidden_page() -> Response {
    error_page(
        StatusCode::FORBIDDEN,
        "403",
        "Forbidden",
        "You don't have permission to perform this action.",
    )
}

/// Self-contained error page (no external assets so it always renders).
pub fn error_page(status: StatusCode, code: &str, title: &str, message: &str) -> Response {
    let html = format!(
        r##"<!doctype html>
<html lang="ja">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{title} - Team</title>
<style>
  :root {{
    --bg: #f6f7f9; --card: #ffffff; --border: #e3e6ea;
    --text: #1a202c; --muted: #5b6472; --primary: #3b82f6;
  }}
  @media (prefers-color-scheme: dark) {{
    :root {{
      --bg: #0f1115; --card: #171a21; --border: #262b36;
      --text: #e7eaf0; --muted: #a3adbf; --primary: #60a5fa;
    }}
  }}
  * {{ box-sizing: border-box; }}
  body {{
    margin: 0; min-height: 100vh; display: flex; align-items: center; justify-content: center;
    background: var(--bg); color: var(--text);
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', 'Hiragino Kaku Gothic ProN', 'Noto Sans JP', sans-serif;
  }}
  .card {{
    text-align: center; padding: 48px 44px; margin: 20px; max-width: 420px; width: 100%;
    background: var(--card); border: 1px solid var(--border); border-radius: 16px;
    box-shadow: 0 4px 14px rgba(0, 0, 0, 0.06);
  }}
  .logo {{
    display: inline-flex; align-items: center; justify-content: center;
    width: 52px; height: 52px; border-radius: 15px; margin-bottom: 18px;
    background: linear-gradient(135deg, #3b82f6, #8b5cf6);
    color: #fff; font-weight: 800; font-size: 26px;
  }}
  .code {{
    font-size: 68px; font-weight: 800; letter-spacing: -0.04em; line-height: 1;
    background: linear-gradient(135deg, #3b82f6, #8b5cf6);
    -webkit-background-clip: text; background-clip: text; color: transparent;
  }}
  h1 {{ margin: 14px 0 8px; font-size: 19px; font-weight: 700; }}
  p {{ margin: 0 0 28px; font-size: 14px; line-height: 1.7; color: var(--muted); }}
  a {{
    display: inline-block; padding: 11px 26px; border-radius: 999px;
    background: var(--primary); color: #fff; font-size: 14px; font-weight: 600; text-decoration: none;
  }}
</style>
</head>
<body>
  <div class="card">
    <div class="logo">T</div>
    <div class="code">{code}</div>
    <h1>{title}</h1>
    <p>{message}</p>
    <a href="/">Back to Feed</a>
  </div>
</body>
</html>"##
    );
    (status, Html(html)).into_response()
}
