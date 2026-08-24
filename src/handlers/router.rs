use axum::extract::DefaultBodyLimit;
use axum::routing::{get, post};
use axum::Router;
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;

use crate::handlers::{account, gist, index, post, tweet};
use crate::state::AppState;

pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/", get(index::index))
        .route("/search", get(index::search))
        .route("/tag", get(index::tag))
        // Account
        .route(
            "/signup",
            get(account::get_signup).post(account::post_signup),
        )
        .route(
            "/signin",
            get(account::get_signin).post(account::post_signin),
        )
        .route("/signout", get(account::signout))
        .route("/auth/google", get(account::auth_google))
        .route(
            "/account/settings",
            get(account::get_settings).post(account::post_settings),
        )
        .route("/account/password", post(account::update_password))
        .route("/account/username", post(account::update_username))
        .route("/account/preference/menu", post(account::preference_menu))
        .route("/account/preference/theme", post(account::preference_theme))
        .route("/account/icon/upload", post(account::icon_upload))
        // Pinned / stocked / draft
        .route("/pinned/list", get(post::pinned_list))
        .route("/stocked/list", get(post::stocked_list))
        .route("/draft/list", get(post::draft_list))
        // Gist
        .route("/gist/new", get(gist::new_form))
        .route("/gist/create", post(gist::create))
        .route("/gist/list", get(gist::list))
        .route("/gist/show/{id}", get(gist::show))
        .route("/gist/edit/{id}", get(gist::edit))
        .route("/gist/update", post(gist::update))
        .route("/gist/delete/{id}", get(gist::delete))
        .route("/gist/comment", post(gist::comment))
        .route("/gist/comment/{id}", post(gist::comment_update))
        // Tweet
        .route("/tweet/list", get(tweet::list))
        .route("/tweet/post", post(tweet::post_tweet))
        .route("/tweet/show/{id}", get(tweet::show))
        .route("/tweet/comment", post(tweet::comment))
        // Tags / notifications / uploads
        .route("/tag/list", get(post::tag_list))
        .route("/notifications", get(post::notifications))
        .route("/notification_count", get(post::notification_count))
        .route("/image/upload", post(post::image_upload))
        // Posts (kind = post | nippo)
        .route("/{kind}/new", get(post::new_form))
        .route("/{kind}/create", post(post::create))
        .route("/{kind}/list", get(post::list))
        .route("/{kind}/show/{id}", get(post::show))
        .route("/{kind}/delete/{id}", get(post::delete))
        .route("/{kind}/edit/{id}", get(post::edit))
        .route("/{kind}/update", post(post::update))
        .route("/{kind}/comment", post(post::comment))
        .route("/{kind}/comment/{id}", post(post::comment_update))
        .route("/{kind}/tags/{id}", post(post::tags_update))
        .route("/{kind}/stock/{id}", post(post::stock))
        .route("/{kind}/unstock/{id}", post(post::unstock))
        .route("/{kind}/share/{id}", post(post::share))
        .route("/{kind}/pin/{id}", post(post::pin))
        .route("/{kind}/unpin/{id}", post(post::unpin))
        // User profile (same first-segment param name as the post routes;
        // handlers extract it positionally)
        .route("/{kind}", get(account::profile_post))
        .route("/{kind}/post", get(account::profile_post))
        .route("/{kind}/nippo", get(account::profile_nippo))
        // Static assets
        .nest_service("/css", ServeDir::new("public/css"))
        .nest_service("/js", ServeDir::new("public/js"))
        .nest_service("/img", ServeDir::new("public/img"))
        .nest_service("/webfonts", ServeDir::new("public/webfonts"))
        .nest_service("/codemirror", ServeDir::new("public/codemirror"))
        .nest_service("/favicons", ServeDir::new("public/favicons"))
        // Allow uploads up to MAX_UPLOAD_BYTES; the small margin lets the
        // multipart handler reject oversized files with a friendly error.
        .layer(DefaultBodyLimit::max(
            crate::handlers::account::MAX_UPLOAD_BYTES + 1024 * 1024,
        ))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
