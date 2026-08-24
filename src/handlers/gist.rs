use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Redirect, Response};
use axum::Form;
use serde::{Deserialize, Serialize};

use crate::auth::AuthUser;
use crate::error::AppResult;
use crate::handlers::PageQuery;
use crate::models;
use crate::models::user::UserWithPreference;
use crate::services;
use crate::state::AppState;
use crate::util::Page;
use crate::views::render;

fn show_path(id: i32) -> String {
    format!("/gist/show/{}", id)
}

pub async fn new_form(
    State(state): State<AppState>,
    AuthUser(login_user): AuthUser,
) -> AppResult<Response> {
    #[derive(Serialize)]
    struct Data {
        logged_in: bool,
        login_user: UserWithPreference,
    }
    let data = Data {
        logged_in: true,
        login_user,
    };
    Ok(render(&state, "gist/form", &data)?.into_response())
}

#[derive(Deserialize)]
pub struct GistForm {
    #[serde(rename = "gist-description")]
    pub description: String,
    #[serde(rename = "gist-filename")]
    pub filename: String,
    #[serde(rename = "gist-code")]
    pub code: String,
}

pub async fn create(
    State(state): State<AppState>,
    AuthUser(login_user): AuthUser,
    Form(form): Form<GistForm>,
) -> AppResult<Response> {
    let id = models::gist::create(
        &state.pool,
        login_user.id,
        &form.description,
        &form.filename,
        &form.code,
    )
    .await?;

    services::notify_slack(
        &state,
        login_user.id,
        services::SlackEvent {
            emoji: ":computer:",
            label: "New Gist".to_string(),
            title: form.description.clone(),
            body: form.code.clone(),
            code: true,
            path: show_path(id),
            mentions: Vec::new(),
        },
    )
    .await;
    Ok(Redirect::to(&show_path(id)).into_response())
}

pub async fn list(
    State(state): State<AppState>,
    AuthUser(login_user): AuthUser,
    Query(q): Query<PageQuery>,
) -> AppResult<Response> {
    let page = Page::new(q.page);
    let gists = models::gist::list(&state.pool, page.offset, page.limit).await?;
    let count = models::gist::count(&state.pool).await?;

    #[derive(Serialize)]
    struct Data {
        logged_in: bool,
        login_user: UserWithPreference,
        gists: Vec<models::gist::Gist>,
        current_page: i32,
        total_page: i32,
        next_page: i32,
        prev_page: i32,
    }

    let data = Data {
        logged_in: true,
        login_user,
        gists,
        current_page: page.current,
        total_page: page.total_pages(count),
        next_page: page.current + 1,
        prev_page: page.current - 1,
    };
    Ok(render(&state, "gist/list", &data)?.into_response())
}

pub async fn show(
    State(state): State<AppState>,
    AuthUser(login_user): AuthUser,
    Path(id): Path<i32>,
) -> AppResult<Response> {
    let gist = models::gist::get_by_id(&state.pool, id).await?;
    let comments = models::gist::get_comments_by_gist_id(&state.pool, id).await?;

    #[derive(Serialize)]
    struct GistComment {
        comment: models::gist::Comment,
        editable: bool,
    }

    let gist_comments: Vec<GistComment> = comments
        .into_iter()
        .map(|comment| GistComment {
            editable: comment.user_id == login_user.id,
            comment,
        })
        .collect();

    #[derive(Serialize)]
    struct Data {
        logged_in: bool,
        login_user: UserWithPreference,
        gist: models::gist::Gist,
        editable: bool,
        deletable: bool,
        comments: Vec<GistComment>,
    }

    let owner = gist.user_id == login_user.id;
    let data = Data {
        logged_in: true,
        login_user,
        editable: owner,
        deletable: owner,
        gist,
        comments: gist_comments,
    };
    Ok(render(&state, "gist/show", &data)?.into_response())
}

pub async fn edit(
    State(state): State<AppState>,
    AuthUser(login_user): AuthUser,
    Path(id): Path<i32>,
) -> AppResult<Response> {
    let gist = models::gist::get_by_id(&state.pool, id).await?;
    if gist.user_id != login_user.id {
        return Ok(StatusCode::FORBIDDEN.into_response());
    }

    #[derive(Serialize)]
    struct Data {
        logged_in: bool,
        login_user: UserWithPreference,
        gist: models::gist::Gist,
    }

    let data = Data {
        logged_in: true,
        login_user,
        gist,
    };
    Ok(render(&state, "gist/edit", &data)?.into_response())
}

#[derive(Deserialize)]
pub struct GistUpdateForm {
    pub id: i32,
    #[serde(rename = "gist-description")]
    pub description: String,
    #[serde(rename = "gist-filename")]
    pub filename: String,
    #[serde(rename = "gist-code")]
    pub code: String,
}

pub async fn update(
    State(state): State<AppState>,
    AuthUser(login_user): AuthUser,
    Form(form): Form<GistUpdateForm>,
) -> AppResult<Response> {
    let old_gist = models::gist::get_by_id(&state.pool, form.id).await?;
    if old_gist.user_id != login_user.id {
        return Ok(StatusCode::FORBIDDEN.into_response());
    }

    models::gist::update(
        &state.pool,
        form.id,
        &form.description,
        &form.filename,
        &form.code,
    )
    .await?;

    services::notify_slack(
        &state,
        login_user.id,
        services::SlackEvent {
            emoji: ":pencil2:",
            label: "Edit Gist".to_string(),
            title: form.description.clone(),
            body: form.code.clone(),
            code: true,
            path: show_path(form.id),
            mentions: Vec::new(),
        },
    )
    .await;
    Ok(Redirect::to(&show_path(form.id)).into_response())
}

pub async fn delete(
    State(state): State<AppState>,
    AuthUser(login_user): AuthUser,
    Path(id): Path<i32>,
) -> AppResult<Response> {
    let gist = models::gist::get_by_id(&state.pool, id).await?;
    if gist.user_id != login_user.id {
        return Ok(StatusCode::FORBIDDEN.into_response());
    }
    models::gist::delete_by_id(&state.pool, id).await?;
    Ok(Redirect::to("/gist/list").into_response())
}

#[derive(Deserialize)]
pub struct CommentForm {
    pub id: i32,
    pub body: String,
}

pub async fn comment(
    State(state): State<AppState>,
    AuthUser(login_user): AuthUser,
    Form(form): Form<CommentForm>,
) -> AppResult<Response> {
    let gist = models::gist::get_by_id(&state.pool, form.id).await?;
    let mut mentions = vec![gist.user.username.clone()];
    for c in models::gist::get_comments_by_gist_id(&state.pool, form.id).await? {
        if !mentions.contains(&c.user.username) {
            mentions.push(c.user.username);
        }
    }

    models::gist::add_comment(&state.pool, login_user.id, form.id, &form.body).await?;
    services::notify_slack(
        &state,
        login_user.id,
        services::SlackEvent {
            emoji: ":speech_balloon:",
            label: "New Comment".to_string(),
            title: gist.description.clone(),
            body: form.body.clone(),
            code: false,
            path: show_path(form.id),
            mentions,
        },
    )
    .await;
    Ok(Redirect::to(&show_path(form.id)).into_response())
}

#[derive(Deserialize)]
pub struct CommentUpdateForm {
    pub action: String,
    pub body: String,
}

pub async fn comment_update(
    State(state): State<AppState>,
    AuthUser(login_user): AuthUser,
    Path(id): Path<i32>,
    Form(form): Form<CommentUpdateForm>,
) -> AppResult<Response> {
    let comment = models::gist::get_comment_by_id(&state.pool, id).await?;
    if comment.user_id != login_user.id {
        return Ok(StatusCode::FORBIDDEN.into_response());
    }
    match form.action.as_str() {
        "update" => {
            models::gist::update_comment_by_id(&state.pool, id, &form.body).await?;
        }
        "delete" => {
            models::gist::delete_comment_by_id(&state.pool, id).await?;
        }
        _ => return Ok(StatusCode::BAD_REQUEST.into_response()),
    }
    Ok(Redirect::to(&show_path(comment.gist_id)).into_response())
}
