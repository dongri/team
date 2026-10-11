use axum::extract::{Multipart, Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Redirect, Response};
use axum::{Form, Json};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::auth::AuthUser;
use crate::error::{AppError, AppResult};
use crate::handlers::account::upload_and_respond;
use crate::handlers::PageQuery;
use crate::models;
use crate::models::user::UserWithPreference;
use crate::services;
use crate::state::AppState;
use crate::util::{self, Page};
use crate::views::render;

fn show_path(kind: &str, id: i32) -> String {
    format!("/{}/show/{}", kind, id)
}

/// Load a post for `login_user_id`. Drafts are private to their author, so
/// anyone else gets a 404 (not a 403, which would confirm the id exists).
async fn load_visible_post(
    state: &AppState,
    id: i32,
    login_user_id: i32,
) -> AppResult<models::post::Post> {
    let post = models::post::get_by_id(&state.pool, id).await?;
    if post.status != "publish" && post.user_id != login_user_id {
        return Err(AppError::NotFound);
    }
    Ok(post)
}

/// The author may always edit. Co-editing (`shared`) opens a published
/// post to every signed-in user.
fn can_edit(post: &models::post::Post, login_user_id: i32) -> bool {
    post.user_id == login_user_id || (post.shared && post.status == "publish")
}

pub async fn new_form(
    State(state): State<AppState>,
    AuthUser(login_user): AuthUser,
    Path(kind): Path<String>,
) -> AppResult<Response> {
    #[derive(Serialize)]
    struct Data {
        logged_in: bool,
        login_user: UserWithPreference,
        kind: String,
        kind_title: String,
    }
    let data = Data {
        logged_in: true,
        login_user,
        kind_title: util::uppercase_first_letter(&kind),
        kind,
    };
    Ok(render(&state, "post/form", &data)?.into_response())
}

#[derive(Deserialize)]
pub struct PostForm {
    pub action: String,
    pub title: String,
    pub body: String,
    pub tags: String,
}

pub async fn create(
    State(state): State<AppState>,
    AuthUser(login_user): AuthUser,
    Path(kind): Path<String>,
    Form(form): Form<PostForm>,
) -> AppResult<Response> {
    let id = models::post::create(
        &state.pool,
        &kind,
        login_user.id,
        &form.action,
        &form.title,
        &form.body,
        &form.tags,
    )
    .await?;

    if form.action == "publish" {
        let (emoji, label) = if kind == "nippo" {
            (":calendar:", "New 日報")
        } else {
            (":memo:", "New Post")
        };
        services::notify_slack(
            &state,
            login_user.id,
            services::SlackEvent {
                emoji,
                label: label.to_string(),
                title: form.title.clone(),
                body: form.body.clone(),
                code: false,
                path: show_path(&kind, id),
                mentions: Vec::new(),
            },
        )
        .await;
    }
    Ok(Redirect::to(&show_path(&kind, id)).into_response())
}

pub async fn list(
    State(state): State<AppState>,
    AuthUser(login_user): AuthUser,
    Path(kind): Path<String>,
    Query(q): Query<PageQuery>,
) -> AppResult<Response> {
    let page = Page::new(q.page);
    let posts = models::post::list(&state.pool, &kind, page.offset, page.limit).await?;
    let count = models::post::count(&state.pool, &kind).await?;

    let new_title = match kind.as_str() {
        "post" => "Post Know-how",
        "nippo" => "Post Nippo",
        _ => "",
    };

    #[derive(Serialize)]
    struct Data {
        logged_in: bool,
        login_user: UserWithPreference,
        posts: Vec<models::post::Post>,
        current_page: i32,
        total_page: i32,
        next_page: i32,
        prev_page: i32,
        kind: String,
        new_title: String,
        kind_title: String,
    }

    let data = Data {
        logged_in: true,
        login_user,
        posts,
        current_page: page.current,
        total_page: page.total_pages(count),
        next_page: page.current + 1,
        prev_page: page.current - 1,
        kind_title: util::uppercase_first_letter(&kind),
        new_title: new_title.to_string(),
        kind,
    };
    Ok(render(&state, "post/list", &data)?.into_response())
}

pub async fn show(
    State(state): State<AppState>,
    AuthUser(login_user): AuthUser,
    Path((kind, id)): Path<(String, i32)>,
) -> AppResult<Response> {
    let post = load_visible_post(&state, id, login_user.id).await?;
    let comments = models::post::get_comments_by_post_id(&state.pool, id).await?;
    let stocked = models::post::is_stocked(&state.pool, login_user.id, id).await?;
    let pinned = models::post::is_pinned(&state.pool, id).await?;

    #[derive(Serialize)]
    struct PostComment {
        comment: models::post::Comment,
        editable: bool,
        kind: String,
    }

    let post_comments: Vec<PostComment> = comments
        .into_iter()
        .map(|comment| PostComment {
            editable: comment.user_id == login_user.id,
            kind: kind.clone(),
            comment,
        })
        .collect();

    #[derive(Serialize)]
    struct Data {
        logged_in: bool,
        login_user: UserWithPreference,
        post: models::post::Post,
        editable: bool,
        deletable: bool,
        shared: bool,
        comments: Vec<PostComment>,
        stocked: bool,
        kind: String,
        kind_title: String,
        pinned: bool,
    }

    let editable = can_edit(&post, login_user.id);
    // Only the author may delete; co-editors get the Edit button alone.
    let deletable = post.user_id == login_user.id;
    let data = Data {
        logged_in: true,
        login_user,
        editable,
        deletable,
        shared: post.shared,
        post,
        comments: post_comments,
        stocked,
        kind_title: util::uppercase_first_letter(&kind),
        kind,
        pinned,
    };
    Ok(render(&state, "post/show", &data)?.into_response())
}

pub async fn delete(
    State(state): State<AppState>,
    AuthUser(login_user): AuthUser,
    Path((kind, id)): Path<(String, i32)>,
) -> AppResult<Response> {
    let post = load_visible_post(&state, id, login_user.id).await?;
    if post.user_id != login_user.id {
        return Err(AppError::Forbidden);
    }
    models::post::delete_by_id(&state.pool, id).await?;
    Ok(Redirect::to(&format!("/{}/list", kind)).into_response())
}

pub async fn edit(
    State(state): State<AppState>,
    AuthUser(login_user): AuthUser,
    Path((kind, id)): Path<(String, i32)>,
) -> AppResult<Response> {
    let post = load_visible_post(&state, id, login_user.id).await?;
    if !can_edit(&post, login_user.id) {
        return Err(AppError::Forbidden);
    }

    let tags = post
        .tags
        .iter()
        .map(|t| t.name.clone())
        .collect::<Vec<_>>()
        .join(",");

    #[derive(Serialize)]
    struct Data {
        logged_in: bool,
        login_user: UserWithPreference,
        post: models::post::Post,
        tags: String,
        kind: String,
        kind_title: String,
    }

    let data = Data {
        logged_in: true,
        login_user,
        post,
        tags,
        kind_title: util::uppercase_first_letter(&kind),
        kind,
    };
    Ok(render(&state, "post/edit", &data)?.into_response())
}

#[derive(Deserialize)]
pub struct PostUpdateForm {
    pub id: i32,
    pub title: String,
    pub body: String,
    pub tags: String,
    pub action: String,
}

fn diff_body(left: &str, right: &str) -> String {
    let mut out = String::new();
    for d in diff::lines(left, right) {
        match d {
            diff::Result::Left(l) => out += &format!("-{}\n", l),
            diff::Result::Both(_, _) => {}
            diff::Result::Right(r) => out += &format!("+{}\n", r),
        }
    }
    out
}

pub async fn update(
    State(state): State<AppState>,
    AuthUser(login_user): AuthUser,
    Path(kind): Path<String>,
    Form(form): Form<PostUpdateForm>,
) -> AppResult<Response> {
    let old_post = load_visible_post(&state, form.id, login_user.id).await?;
    if !can_edit(&old_post, login_user.id) {
        return Err(AppError::Forbidden);
    }
    if form.action != "publish" && form.action != "draft" {
        return Ok(StatusCode::BAD_REQUEST.into_response());
    }

    models::post::update(
        &state.pool,
        form.id,
        &form.title,
        &form.body,
        &form.tags,
        &form.action,
    )
    .await?;

    if form.action == "publish" {
        // Publishing a draft is effectively a new post; otherwise show the diff.
        let was_draft = old_post.status == "draft";
        let (emoji, label) = match (kind == "nippo", was_draft) {
            (true, true) => (":calendar:", "New 日報"),
            (true, false) => (":pencil2:", "Edit 日報"),
            (false, true) => (":memo:", "New Post"),
            (false, false) => (":pencil2:", "Edit Post"),
        };
        let body = if was_draft {
            form.body.clone()
        } else {
            diff_body(&old_post.body, &form.body)
        };
        services::notify_slack(
            &state,
            login_user.id,
            services::SlackEvent {
                emoji,
                label: label.to_string(),
                title: form.title.clone(),
                body,
                code: !was_draft,
                path: show_path(&kind, form.id),
                mentions: Vec::new(),
            },
        )
        .await;
    }
    Ok(Redirect::to(&show_path(&kind, form.id)).into_response())
}

#[derive(Deserialize)]
pub struct TagsUpdateForm {
    pub id: i32,
    pub tags: String,
}

pub async fn tags_update(
    State(state): State<AppState>,
    AuthUser(login_user): AuthUser,
    Path((kind, id)): Path<(String, i32)>,
    Form(form): Form<TagsUpdateForm>,
) -> AppResult<Response> {
    // The path id is authoritative; ignore a mismatching form id.
    if form.id != id {
        return Ok(StatusCode::BAD_REQUEST.into_response());
    }
    let old_post = load_visible_post(&state, id, login_user.id).await?;
    if !can_edit(&old_post, login_user.id) {
        return Err(AppError::Forbidden);
    }
    // Keep the current status: retagging must never publish a draft.
    models::post::update(
        &state.pool,
        form.id,
        &old_post.title,
        &old_post.body,
        &form.tags,
        &old_post.status,
    )
    .await?;

    services::notify_slack(
        &state,
        login_user.id,
        services::SlackEvent {
            emoji: ":label:",
            label: "Update Tags".to_string(),
            title: old_post.title.clone(),
            body: form.tags.clone(),
            code: false,
            path: show_path(&kind, form.id),
            mentions: Vec::new(),
        },
    )
    .await;
    Ok(Redirect::to(&show_path(&kind, form.id)).into_response())
}

#[derive(Deserialize)]
pub struct CommentForm {
    pub id: i32,
    pub body: String,
}

pub async fn comment(
    State(state): State<AppState>,
    AuthUser(login_user): AuthUser,
    Path(kind): Path<String>,
    Form(form): Form<CommentForm>,
) -> AppResult<Response> {
    let post = load_visible_post(&state, form.id, login_user.id).await?;
    let mut mentions = vec![post.user.username.clone()];
    for c in models::post::get_comments_by_post_id(&state.pool, form.id).await? {
        if !mentions.contains(&c.user.username) {
            mentions.push(c.user.username);
        }
    }

    models::post::add_comment(&state.pool, login_user.id, form.id, &form.body).await?;
    services::notify_slack(
        &state,
        login_user.id,
        services::SlackEvent {
            emoji: ":speech_balloon:",
            label: "New Comment".to_string(),
            title: post.title.clone(),
            body: form.body.clone(),
            code: false,
            path: show_path(&kind, form.id),
            mentions,
        },
    )
    .await;
    Ok(Redirect::to(&show_path(&kind, form.id)).into_response())
}

#[derive(Deserialize)]
pub struct CommentUpdateForm {
    pub action: String,
    pub body: String,
}

pub async fn comment_update(
    State(state): State<AppState>,
    AuthUser(login_user): AuthUser,
    Path((kind, id)): Path<(String, i32)>,
    Form(form): Form<CommentUpdateForm>,
) -> AppResult<Response> {
    let comment = models::post::get_comment_by_id(&state.pool, id).await?;
    if comment.user_id != login_user.id {
        return Ok(StatusCode::FORBIDDEN.into_response());
    }
    match form.action.as_str() {
        "update" => {
            models::post::update_comment_by_id(&state.pool, id, &form.body).await?;
        }
        "delete" => {
            models::post::delete_comment_by_id(&state.pool, id).await?;
        }
        _ => return Ok(StatusCode::BAD_REQUEST.into_response()),
    }
    Ok(Redirect::to(&show_path(&kind, comment.post_id)).into_response())
}

pub async fn stock(
    State(state): State<AppState>,
    AuthUser(login_user): AuthUser,
    Path((kind, id)): Path<(String, i32)>,
) -> AppResult<Response> {
    load_visible_post(&state, id, login_user.id).await?;
    models::post::stock_post(&state.pool, login_user.id, id).await?;
    Ok(Redirect::to(&show_path(&kind, id)).into_response())
}

pub async fn unstock(
    State(state): State<AppState>,
    AuthUser(login_user): AuthUser,
    Path((kind, id)): Path<(String, i32)>,
) -> AppResult<Response> {
    models::post::stock_remove(&state.pool, login_user.id, id).await?;
    Ok(Redirect::to(&show_path(&kind, id)).into_response())
}

pub async fn stocked_list(
    State(state): State<AppState>,
    AuthUser(login_user): AuthUser,
    Query(q): Query<PageQuery>,
) -> AppResult<Response> {
    let page = Page::new(q.page);
    let posts =
        models::post::stocked_list(&state.pool, login_user.id, page.offset, page.limit).await?;
    let count = models::post::stocked_count(&state.pool, login_user.id).await?;

    #[derive(Serialize)]
    struct Data {
        logged_in: bool,
        login_user: UserWithPreference,
        posts: Vec<models::post::Post>,
        current_page: i32,
        total_page: i32,
        next_page: i32,
        prev_page: i32,
    }

    let data = Data {
        logged_in: true,
        login_user,
        posts,
        current_page: page.current,
        total_page: page.total_pages(count),
        next_page: page.current + 1,
        prev_page: page.current - 1,
    };
    Ok(render(&state, "stock/list", &data)?.into_response())
}

/// Open a post for co-editing. Only the author can grant this, since it
/// lets every signed-in user rewrite the post.
pub async fn share(
    State(state): State<AppState>,
    AuthUser(login_user): AuthUser,
    Path((kind, id)): Path<(String, i32)>,
) -> AppResult<Response> {
    let post = load_visible_post(&state, id, login_user.id).await?;
    if post.user_id != login_user.id {
        return Err(AppError::Forbidden);
    }
    if post.status != "publish" {
        return Ok(StatusCode::BAD_REQUEST.into_response());
    }
    models::post::share_post(&state.pool, id).await?;
    Ok(Redirect::to(&show_path(&kind, id)).into_response())
}

/// Pinning is a team-wide feature: any member may pin a published post.
pub async fn pin(
    State(state): State<AppState>,
    AuthUser(login_user): AuthUser,
    Path((kind, id)): Path<(String, i32)>,
) -> AppResult<Response> {
    let post = load_visible_post(&state, id, login_user.id).await?;
    if post.status != "publish" {
        return Ok(StatusCode::BAD_REQUEST.into_response());
    }
    if !models::post::is_pinned(&state.pool, id).await? {
        models::post::pin_post(&state.pool, login_user.id, id).await?;
    }
    Ok(Redirect::to(&show_path(&kind, id)).into_response())
}

/// Only the post's author or whoever pinned it may unpin.
pub async fn unpin(
    State(state): State<AppState>,
    AuthUser(login_user): AuthUser,
    Path((kind, id)): Path<(String, i32)>,
) -> AppResult<Response> {
    let post = load_visible_post(&state, id, login_user.id).await?;
    let pinned_by_me = models::post::is_pinned_by(&state.pool, login_user.id, id).await?;
    if post.user_id != login_user.id && !pinned_by_me {
        return Err(AppError::Forbidden);
    }
    models::post::unpin_post(&state.pool, id).await?;
    Ok(Redirect::to(&show_path(&kind, id)).into_response())
}

pub async fn pinned_list(
    State(state): State<AppState>,
    AuthUser(login_user): AuthUser,
    Query(q): Query<PageQuery>,
) -> AppResult<Response> {
    let page = Page::new(q.page);
    let pinneds = models::post::pinned_list(&state.pool, page.offset, page.limit).await?;
    let count = models::post::pinned_count(&state.pool).await?;

    #[derive(Serialize)]
    struct Data {
        logged_in: bool,
        login_user: UserWithPreference,
        pinneds: Vec<models::post::Post>,
        current_page: i32,
        total_page: i32,
        next_page: i32,
        prev_page: i32,
    }

    let data = Data {
        logged_in: true,
        login_user,
        pinneds,
        current_page: page.current,
        total_page: page.total_pages(count),
        next_page: page.current + 1,
        prev_page: page.current - 1,
    };
    Ok(render(&state, "pinned/list", &data)?.into_response())
}

pub async fn draft_list(
    State(state): State<AppState>,
    AuthUser(login_user): AuthUser,
) -> AppResult<Response> {
    let posts = models::post::draft_list(&state.pool, login_user.id).await?;

    #[derive(Serialize)]
    struct Data {
        logged_in: bool,
        login_user: UserWithPreference,
        posts: Vec<models::post::Post>,
    }

    let data = Data {
        logged_in: true,
        login_user,
        posts,
    };
    Ok(render(&state, "draft/list", &data)?.into_response())
}

pub async fn tag_list(
    State(state): State<AppState>,
    AuthUser(login_user): AuthUser,
) -> AppResult<Response> {
    let tags = models::tag::tag_list(&state.pool).await?;

    #[derive(Serialize)]
    struct Data {
        logged_in: bool,
        login_user: UserWithPreference,
        tags: Vec<models::tag::Tag>,
    }

    let data = Data {
        logged_in: true,
        login_user,
        tags,
    };
    Ok(render(&state, "tag/list", &data)?.into_response())
}

pub async fn notifications(
    State(state): State<AppState>,
    AuthUser(login_user): AuthUser,
    Query(q): Query<PageQuery>,
) -> AppResult<Response> {
    let page = Page::new(q.page);
    let notifications =
        models::notification::list(&state.pool, login_user.id, page.offset, page.limit).await?;
    let count = models::notification::count(&state.pool, login_user.id).await?;

    #[derive(Serialize)]
    struct Data {
        logged_in: bool,
        login_user: UserWithPreference,
        notifications: Vec<models::notification::Notification>,
        current_page: i32,
        total_page: i32,
        next_page: i32,
        prev_page: i32,
    }

    let data = Data {
        logged_in: true,
        login_user,
        notifications,
        current_page: page.current,
        total_page: page.total_pages(count),
        next_page: page.current + 1,
        prev_page: page.current - 1,
    };
    Ok(render(&state, "notifications", &data)?.into_response())
}

pub async fn notification_count(
    State(state): State<AppState>,
    AuthUser(login_user): AuthUser,
) -> AppResult<Response> {
    let count = models::notification::unread_count(&state.pool, login_user.id).await?;
    Ok(Json(json!({ "count": count })).into_response())
}

pub async fn image_upload(
    State(state): State<AppState>,
    AuthUser(_login_user): AuthUser,
    multipart: Multipart,
) -> AppResult<Response> {
    Ok(upload_and_respond(&state, multipart, "public/img/posts").await)
}
