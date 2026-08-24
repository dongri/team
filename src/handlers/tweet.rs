use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Redirect, Response};
use axum::{Form, Json};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::auth::AuthUser;
use crate::error::AppResult;
use crate::handlers::PageQuery;
use crate::models;
use crate::models::user::UserWithPreference;
use crate::services;
use crate::state::AppState;
use crate::util::Page;
use crate::views::render;

pub async fn list(
    State(state): State<AppState>,
    AuthUser(login_user): AuthUser,
    Query(q): Query<PageQuery>,
) -> AppResult<Response> {
    let page = Page::new(q.page);
    let tweets = models::tweet::list(&state.pool, login_user.id, page.offset, page.limit).await?;
    let count = models::tweet::count(&state.pool).await?;

    #[derive(Serialize)]
    struct Data {
        logged_in: bool,
        login_user: UserWithPreference,
        tweets: Vec<models::tweet::Tweet>,
        current_page: i32,
        total_page: i32,
        next_page: i32,
        prev_page: i32,
    }

    let data = Data {
        logged_in: true,
        login_user,
        tweets,
        current_page: page.current,
        total_page: page.total_pages(count),
        next_page: page.current + 1,
        prev_page: page.current - 1,
    };
    Ok(render(&state, "tweet/list", &data)?.into_response())
}

#[derive(Deserialize)]
pub struct TweetForm {
    pub body: String,
}

pub async fn post_tweet(
    State(state): State<AppState>,
    AuthUser(login_user): AuthUser,
    Form(form): Form<TweetForm>,
) -> AppResult<Response> {
    let id = models::tweet::create(&state.pool, login_user.id, &form.body).await?;
    services::notify_slack(
        &state,
        login_user.id,
        services::SlackEvent {
            emoji: ":bird:",
            label: "New Tweet".to_string(),
            title: String::new(),
            body: form.body.clone(),
            code: false,
            path: format!("/tweet/show/{}", id),
            mentions: Vec::new(),
        },
    )
    .await;
    Ok(Redirect::to("/tweet/list").into_response())
}

pub async fn show(
    State(state): State<AppState>,
    AuthUser(login_user): AuthUser,
    Path(id): Path<i32>,
) -> AppResult<Response> {
    let tweet = models::tweet::get_by_id(&state.pool, login_user.id, id).await?;
    let comments = models::tweet::get_comments_by_tweet_id(&state.pool, id).await?;

    #[derive(Serialize)]
    struct TweetComment {
        comment: models::tweet::Comment,
        editable: bool,
    }

    let tweet_comments: Vec<TweetComment> = comments
        .into_iter()
        .map(|comment| TweetComment {
            editable: comment.user_id == login_user.id,
            comment,
        })
        .collect();

    #[derive(Serialize)]
    struct Data {
        logged_in: bool,
        login_user: UserWithPreference,
        tweet: models::tweet::Tweet,
        comments: Vec<TweetComment>,
    }

    let data = Data {
        logged_in: true,
        login_user,
        tweet,
        comments: tweet_comments,
    };
    Ok(render(&state, "tweet/show", &data)?.into_response())
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
    let tweet = models::tweet::get_by_id(&state.pool, login_user.id, form.id).await?;
    let mut mentions = vec![tweet.user.username.clone()];
    for c in models::tweet::get_comments_by_tweet_id(&state.pool, form.id).await? {
        if !mentions.contains(&c.user.username) {
            mentions.push(c.user.username);
        }
    }

    models::tweet::add_comment(&state.pool, login_user.id, form.id, &form.body).await?;
    let tweet_excerpt: String = tweet.body.chars().take(40).collect();
    services::notify_slack(
        &state,
        login_user.id,
        services::SlackEvent {
            emoji: ":speech_balloon:",
            label: "New Comment".to_string(),
            title: tweet_excerpt,
            body: form.body.clone(),
            code: false,
            path: format!("/tweet/show/{}", form.id),
            mentions,
        },
    )
    .await;
    Ok(Redirect::to(&format!("/tweet/show/{}", form.id)).into_response())
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
    let comment = models::tweet::get_comment_by_id(&state.pool, id).await?;
    if comment.user_id != login_user.id {
        return Ok(StatusCode::FORBIDDEN.into_response());
    }
    match form.action.as_str() {
        "update" => {
            models::tweet::update_comment_by_id(&state.pool, id, &form.body).await?;
        }
        "delete" => {
            models::tweet::delete_comment_by_id(&state.pool, id).await?;
        }
        _ => return Ok(StatusCode::BAD_REQUEST.into_response()),
    }
    Ok(Redirect::to(&format!("/tweet/show/{}", comment.tweet_id)).into_response())
}

pub async fn like(
    State(state): State<AppState>,
    AuthUser(login_user): AuthUser,
    Path(id): Path<i32>,
) -> AppResult<Response> {
    let liked = models::tweet::toggle_like(&state.pool, login_user.id, id).await?;
    let like_count = models::tweet::like_count(&state.pool, id).await?;
    Ok(Json(json!({ "liked": liked, "like_count": like_count })).into_response())
}
