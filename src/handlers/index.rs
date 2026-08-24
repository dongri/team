use axum::extract::{Query, State};
use axum::response::Html;
use serde::{Deserialize, Serialize};

use crate::auth::AuthUser;
use crate::error::AppResult;
use crate::handlers::PageQuery;
use crate::models;
use crate::state::AppState;
use crate::util::{self, Page};
use crate::views::render;

pub async fn index(
    State(state): State<AppState>,
    AuthUser(login_user): AuthUser,
    Query(q): Query<PageQuery>,
) -> AppResult<Html<String>> {
    let page = Page::new(q.page);

    let feeds = models::post::get_feeds(&state.pool, page.offset, page.limit).await?;
    let count = models::post::get_feed_count(&state.pool).await?;
    let pinneds = models::post::pinned_list(&state.pool, 0, 3).await?;

    #[derive(Serialize)]
    struct Data {
        logged_in: bool,
        login_user: models::user::UserWithPreference,
        feeds: Vec<models::post::Feed>,
        current_page: i32,
        total_page: i32,
        next_page: i32,
        prev_page: i32,
        pinneds: Vec<models::post::Post>,
    }

    let data = Data {
        logged_in: true,
        login_user,
        feeds,
        current_page: page.current,
        total_page: page.total_pages(count),
        next_page: page.current + 1,
        prev_page: page.current - 1,
        pinneds,
    };
    render(&state, "index", &data)
}

#[derive(Deserialize, Debug)]
pub struct SearchQuery {
    pub keyword: Option<String>,
    pub kind: Option<String>,
    pub page: Option<i32>,
}

pub async fn search(
    State(state): State<AppState>,
    AuthUser(login_user): AuthUser,
    Query(q): Query<SearchQuery>,
) -> AppResult<Html<String>> {
    let keyword = q.keyword.unwrap_or_default();
    let kind = q.kind.unwrap_or_else(|| "all".to_string());
    let page = Page::new(q.page);

    let posts = models::post::search(&state.pool, &keyword, &kind, page.offset, page.limit).await?;
    let count = models::post::search_count(&state.pool, &keyword, &kind).await?;

    #[derive(Serialize)]
    struct Data {
        logged_in: bool,
        login_user: models::user::UserWithPreference,
        posts: Vec<models::post::Post>,
        current_page: i32,
        total_page: i32,
        next_page: i32,
        prev_page: i32,
        keyword: String,
        kind: String,
        kind_all_active: String,
        kind_post_active: String,
        kind_nippo_active: String,
        kind_title: String,
    }

    let active = |k: &str| {
        if kind == k {
            "is-active".to_string()
        } else {
            String::new()
        }
    };

    let data = Data {
        logged_in: true,
        login_user,
        posts,
        current_page: page.current,
        total_page: page.total_pages(count),
        next_page: page.current + 1,
        prev_page: page.current - 1,
        kind_all_active: active("all"),
        kind_post_active: active("post"),
        kind_nippo_active: active("nippo"),
        kind_title: util::uppercase_first_letter(&kind),
        keyword,
        kind,
    };
    render(&state, "search", &data)
}

#[derive(Deserialize, Debug)]
pub struct TagQuery {
    pub name: String,
    pub page: Option<i32>,
}

pub async fn tag(
    State(state): State<AppState>,
    AuthUser(login_user): AuthUser,
    Query(q): Query<TagQuery>,
) -> AppResult<Html<String>> {
    let page = Page::new(q.page);

    let posts = models::tag::tag_search(&state.pool, &q.name, page.offset, page.limit).await?;
    let count = models::tag::tag_count(&state.pool, &q.name).await?;

    #[derive(Serialize)]
    struct Data {
        logged_in: bool,
        login_user: models::user::UserWithPreference,
        posts: Vec<models::post::Post>,
        current_page: i32,
        total_page: i32,
        next_page: i32,
        prev_page: i32,
        tag_name: String,
    }

    let data = Data {
        logged_in: true,
        login_user,
        posts,
        current_page: page.current,
        total_page: page.total_pages(count),
        next_page: page.current + 1,
        prev_page: page.current - 1,
        tag_name: q.name,
    };
    render(&state, "tag", &data)
}
