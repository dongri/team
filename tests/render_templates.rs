use handlebars::{DirectorySourceOptions, Handlebars};
use serde_json::{json, Value};

fn registry() -> Handlebars<'static> {
    let mut hb = Handlebars::new();
    hb.register_templates_directory("./templates", DirectorySourceOptions::default())
        .expect("register templates");
    hb
}

fn user() -> Value {
    json!({
        "id": 1,
        "username": "alice",
        "icon_url": null,
        "username_hash": "abcdef0123456789",
    })
}

fn login_user() -> Value {
    json!({
        "id": 1,
        "username": "alice",
        "icon_url": null,
        "username_hash": "abcdef0123456789",
        "menu": ["post", "nippo", "tag", "gist", "tweet"],
        "theme": "light",
    })
}

fn post() -> Value {
    json!({
        "id": 10,
        "kind": "post",
        "user_id": 1,
        "title": "Hello world",
        "body": "# Hi\nbody text",
        "created": "2026-01-01T00:00:00",
        "formated_created": "2026-01-01 09:00:00",
        "user": user(),
        "tags": [{"id": 1, "name": "rust"}],
        "shared": false,
        "status": "publish",
    })
}

fn pagination() -> Vec<(&'static str, Value)> {
    vec![
        ("current_page", json!(1)),
        ("total_page", json!(3)),
        ("next_page", json!(2)),
        ("prev_page", json!(0)),
    ]
}

fn base(mut data: Value) -> Value {
    let obj = data.as_object_mut().unwrap();
    obj.insert("logged_in".into(), json!(true));
    obj.insert("login_user".into(), login_user());
    for (k, v) in pagination() {
        obj.entry(k).or_insert(v);
    }
    data
}

#[test]
fn render_all_pages() {
    let hb = registry();

    let comment = json!({
        "id": 5,
        "user_id": 1,
        "post_id": 10,
        "body": "nice post",
        "user": user(),
    });

    let cases: Vec<(&str, Value)> = vec![
        (
            "index",
            base(json!({
                "feeds": [{
                    "id": 10, "post": "post", "comment": "", "gist": "",
                    "kind": "post", "user_id": 1, "title": "Hello", "body": "",
                    "formated_created": "2026-01-01 09:00:00",
                    "user": user(), "tags": [{"id":1,"name":"rust"}],
                }],
                "pinneds": [post()],
            })),
        ),
        (
            "search",
            base(json!({
                "posts": [post()],
                "keyword": "hello",
                "kind": "all",
                "kind_all_active": "is-active",
                "kind_post_active": "",
                "kind_nippo_active": "",
                "kind_title": "All",
            })),
        ),
        ("tag", base(json!({"posts": [post()], "tag_name": "rust"}))),
        ("tag/list", base(json!({"tags": [{"id":1,"name":"rust"}]}))),
        (
            "notifications",
            base(json!({
                "notifications": [{
                    "id": 1, "path": "/post/show/10", "from_user": 1, "to_user": 2,
                    "body": "hi", "read": false,
                    "formated_created": "2026-01-01 09:00:00", "user": user(),
                }],
            })),
        ),
        ("pinned/list", base(json!({"pinneds": [post()]}))),
        ("stock/list", base(json!({"posts": [post()]}))),
        ("draft/list", base(json!({"posts": [post()]}))),
        ("account/signin", json!({})),
        ("account/signup", json!({})),
        ("account/signin", json!({"google_enabled": true})),
        (
            "account/settings",
            base(json!({
                "user": user(),
                "menu": [{"name": "post", "checked": "checked"}],
                "theme_light": "checked",
                "theme_black": "",
            })),
        ),
        (
            "account/profile",
            base(json!({
                "user": user(),
                "posts": [post()],
                "kind": "post",
                "kind_post_active": "is-active",
                "kind_nippo_active": "",
            })),
        ),
        (
            "post/list",
            base(json!({
                "posts": [post()],
                "kind": "post",
                "kind_title": "Post",
                "new_title": "Post Know-how",
            })),
        ),
        (
            "post/show",
            base(json!({
                "post": post(),
                "editable": true,
                "deletable": true,
                "shared": false,
                "stocked": false,
                "pinned": false,
                "kind": "post",
                "kind_title": "Post",
                "comments": [{"comment": comment, "editable": true, "kind": "post"}],
            })),
        ),
        (
            "post/form",
            base(json!({"kind": "post", "kind_title": "Post"})),
        ),
        (
            "post/edit",
            base(json!({
                "post": post(),
                "tags": "rust,tips",
                "kind": "post",
                "kind_title": "Post",
            })),
        ),
        (
            "gist/list",
            base(json!({
                "gists": [{
                    "id": 3, "user_id": 1, "description": "desc", "filename": "a.rs",
                    "code": "fn main() {}", "formated_created": "2026-01-01 09:00:00",
                    "user": user(),
                }],
            })),
        ),
        (
            "gist/show",
            base(json!({
                "gist": {
                    "id": 3, "user_id": 1, "description": "desc", "filename": "a.rs",
                    "code": "fn main() {}", "formated_created": "2026-01-01 09:00:00",
                    "user": user(),
                },
                "editable": true,
                "deletable": true,
                "comments": [{"comment": {
                    "id": 5, "user_id": 1, "gist_id": 3, "body": "nice", "user": user(),
                }, "editable": true}],
            })),
        ),
        ("gist/form", base(json!({}))),
        (
            "gist/edit",
            base(json!({
                "gist": {
                    "id": 3, "user_id": 1, "description": "desc", "filename": "a.rs",
                    "code": "fn main() {}", "formated_created": "2026-01-01 09:00:00",
                    "user": user(),
                },
            })),
        ),
        (
            "tweet/list",
            base(json!({
                "tweets": [{
                    "id": 7, "user_id": 1, "body": "hello",
                    "formated_created": "2026-01-01 09:00:00",
                    "comment_count": 2, "user": user(),
                }],
            })),
        ),
        (
            "tweet/show",
            base(json!({
                "tweet": {
                    "id": 7, "user_id": 1, "body": "hello",
                    "formated_created": "2026-01-01 09:00:00",
                    "comment_count": 2, "user": user(),
                },
                "comments": [{
                    "id": 8, "user_id": 1, "tweet_id": 7, "body": "yo",
                    "formated_created": "2026-01-01 09:00:00", "user": user(),
                }],
            })),
        ),
    ];

    for (name, data) in cases {
        let html = hb
            .render(name, &data)
            .unwrap_or_else(|e| panic!("failed to render {}: {}", name, e));
        assert!(
            html.contains("<!doctype html>"),
            "{} did not render the layout",
            name
        );
        // Auth pages suppress the site header; every other page shows it.
        let has_header = html.contains("class=\"nav\"");
        if name.starts_with("account/sign") {
            assert!(!has_header, "{} should not render the header", name);
            // Google sign-in appears only when configured.
            let expects_google = data
                .get("google_enabled")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            assert_eq!(
                html.contains("/auth/google"),
                expects_google,
                "{} google button visibility mismatch",
                name
            );
        } else {
            assert!(has_header, "{} should render the header", name);
        }
    }
}
