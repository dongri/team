use axum::response::Html;
use serde::Serialize;

use crate::error::AppResult;
use crate::state::AppState;

pub fn render<T: Serialize>(state: &AppState, name: &str, data: &T) -> AppResult<Html<String>> {
    Ok(Html(state.templates.render(name, data)?))
}
