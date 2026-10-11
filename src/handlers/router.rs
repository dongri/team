use std::net::SocketAddr;
use std::time::Instant;

use axum::extract::{ConnectInfo, DefaultBodyLimit, Request, State};
use axum::http::header::{HeaderValue, ORIGIN, REFERER};
use axum::http::{HeaderMap, Method};
use axum::middleware::Next;
use axum::response::Response;
use axum::routing::{get, post};
use axum::Router;
use tower_http::services::ServeDir;

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
        .route("/signout", post(account::signout))
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
        .route("/gist/delete/{id}", post(gist::delete))
        .route("/gist/comment", post(gist::comment))
        .route("/gist/comment/{id}", post(gist::comment_update))
        // Tweet
        .route("/tweet/list", get(tweet::list))
        .route("/tweet/post", post(tweet::post_tweet))
        .route("/tweet/show/{id}", get(tweet::show))
        .route("/tweet/comment", post(tweet::comment))
        .route("/tweet/comment/{id}", post(tweet::comment_update))
        .route("/tweet/like/{id}", post(tweet::like))
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
        .route("/{kind}/delete/{id}", post(post::delete))
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
        .route("/{kind}/gist", get(account::profile_gist))
        .route("/{kind}/tweet", get(account::profile_tweet))
        // Static assets
        .nest_service("/css", ServeDir::new("public/css"))
        .nest_service("/js", ServeDir::new("public/js"))
        .nest_service("/img", ServeDir::new("public/img"))
        .nest_service("/webfonts", ServeDir::new("public/webfonts"))
        .nest_service("/codemirror", ServeDir::new("public/codemirror"))
        .nest_service("/favicons", ServeDir::new("public/favicons"))
        .fallback(not_found)
        // Allow uploads up to MAX_UPLOAD_BYTES; the small margin lets the
        // multipart handler reject oversized files with a friendly error.
        .layer(DefaultBodyLimit::max(
            crate::handlers::account::MAX_UPLOAD_BYTES + 1024 * 1024,
        ))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            csrf_guard,
        ))
        .layer(axum::middleware::from_fn(security_headers))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            access_log,
        ))
        .with_state(state)
}

async fn not_found() -> Response {
    crate::error::not_found_page()
}

/// Reject state-changing requests that did not originate from this site.
///
/// All mutating endpoints are cookie-authenticated HTML forms or same-origin
/// `fetch` calls, so a legitimate browser request always carries Fetch
/// Metadata (`Sec-Fetch-Site`) and/or an `Origin`/`Referer` matching our own
/// host. Together with `SameSite=Lax` cookies this blocks cross-site request
/// forgery without a per-form token.
async fn csrf_guard(State(state): State<AppState>, req: Request, next: Next) -> Response {
    if matches!(
        *req.method(),
        Method::GET | Method::HEAD | Method::OPTIONS | Method::TRACE
    ) {
        return next.run(req).await;
    }

    let allowed = request_is_same_origin(req.headers(), state.config.domain_authority().as_deref());
    if !allowed {
        tracing::warn!(
            target: "team::csrf",
            "rejected cross-site {} {}",
            req.method(),
            req.uri().path()
        );
        return crate::error::forbidden_page();
    }
    next.run(req).await
}

/// Pure check behind `csrf_guard`, split out for testing.
fn request_is_same_origin(headers: &HeaderMap, configured_authority: Option<&str>) -> bool {
    // Fetch Metadata is set by the browser and cannot be spoofed by pages.
    if let Some(site) = headers.get("sec-fetch-site").and_then(|v| v.to_str().ok()) {
        return matches!(site, "same-origin" | "none");
    }

    // Older browsers: fall back to Origin, then Referer. Absence of both on
    // a POST is not something a browser form submission produces.
    let source = headers
        .get(ORIGIN)
        .or_else(|| headers.get(REFERER))
        .and_then(|v| v.to_str().ok());
    let Some(source) = source else {
        return false;
    };
    let Some(source_authority) = authority_of(source) else {
        return false;
    };

    let host_header = headers
        .get(axum::http::header::HOST)
        .and_then(|v| v.to_str().ok())
        .map(|h| h.trim().to_ascii_lowercase());

    host_header.as_deref() == Some(source_authority.as_str())
        || configured_authority
            .map(|a| a.eq_ignore_ascii_case(&source_authority))
            .unwrap_or(false)
}

/// `host[:port]` of an absolute URL, lower-cased, default ports omitted.
fn authority_of(url: &str) -> Option<String> {
    let parsed = url::Url::parse(url).ok()?;
    let host = parsed.host_str()?.to_ascii_lowercase();
    Some(match parsed.port() {
        Some(port) => format!("{}:{}", host, port),
        None => host,
    })
}

/// Defensive response headers for every response.
async fn security_headers(req: Request, next: Next) -> Response {
    let mut response = next.run(req).await;
    let headers = response.headers_mut();
    headers.insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );
    headers.insert("x-frame-options", HeaderValue::from_static("DENY"));
    headers.insert(
        "referrer-policy",
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );
    response
}

/// One access-log line per request:
/// `<client> "<method> <path>" <status> <latency>ms`
async fn access_log(State(state): State<AppState>, req: Request, next: Next) -> Response {
    let method = req.method().clone();
    // The OAuth callback carries a one-time authorization code in its query
    // string; keep it (and any other query) out of the logs.
    let target = match req.uri().path() {
        p if p.starts_with("/auth/") => p.to_string(),
        _ => req.uri().to_string(),
    };
    let peer = req
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|ci| ci.0);
    let client = crate::util::client_ip(req.headers(), peer, state.config.team_trust_proxy);

    let start = Instant::now();
    let response = next.run(req).await;

    tracing::info!(
        target: "team::access",
        "{} \"{} {}\" {} {}ms",
        client,
        method,
        target,
        response.status().as_u16(),
        start.elapsed().as_millis()
    );
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    fn headers(pairs: &[(&str, &str)]) -> HeaderMap {
        let mut h = HeaderMap::new();
        for (k, v) in pairs {
            h.insert(
                axum::http::HeaderName::from_bytes(k.as_bytes()).unwrap(),
                v.parse().unwrap(),
            );
        }
        h
    }

    #[test]
    fn fetch_metadata_decides_when_present() {
        assert!(request_is_same_origin(
            &headers(&[("sec-fetch-site", "same-origin")]),
            None
        ));
        assert!(request_is_same_origin(
            &headers(&[("sec-fetch-site", "none")]),
            None
        ));
        assert!(!request_is_same_origin(
            &headers(&[
                ("sec-fetch-site", "cross-site"),
                ("origin", "http://localhost:3000"),
                ("host", "localhost:3000"),
            ]),
            None
        ));
        assert!(!request_is_same_origin(
            &headers(&[("sec-fetch-site", "same-site")]),
            None
        ));
    }

    #[test]
    fn origin_must_match_host_or_configured_domain() {
        assert!(request_is_same_origin(
            &headers(&[
                ("origin", "http://localhost:3000"),
                ("host", "localhost:3000")
            ]),
            None
        ));
        assert!(request_is_same_origin(
            &headers(&[
                ("origin", "https://team.example.com"),
                ("host", "internal:3000")
            ]),
            Some("team.example.com")
        ));
        assert!(!request_is_same_origin(
            &headers(&[("origin", "https://evil.com"), ("host", "team.example.com")]),
            Some("team.example.com")
        ));
        assert!(request_is_same_origin(
            &headers(&[
                ("referer", "https://team.example.com/post/show/1"),
                ("host", "team.example.com"),
            ]),
            None
        ));
        assert!(!request_is_same_origin(&headers(&[("host", "x")]), None));
        assert!(!request_is_same_origin(
            &headers(&[("origin", "null"), ("host", "x")]),
            None
        ));
    }
}
