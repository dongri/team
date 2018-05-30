use std::path::Path;

use mount::Mount;
use staticfile::Static;

use Router;
use handlers::index;
use handlers::account;
use handlers::post;
use handlers::gist;
use handlers::tweet;

pub fn create_router() -> Router {
    let mut router = Router::new();
    router.get("/", index::index_handler, "index");
    router.get("/search", index::search_handler, "search");
    router.get("/tag", index::tag_handler, "tag");

    router.get("/signup", account::get_signup_handler, "account/get_signup");
    router.post("/signup", account::post_signup_handler, "account/post_signup");
    router.get("/signin", account::get_signin_handler, "account/get_signin");
    router.post("/signin", account::post_signin_handler, "account/post_signin");
    router.get("/signout", account::get_signout_handler, "account/get_signout");

    router.get("/auth/google", account::get_auth_google_handler, "account/get_auth_google");

    router.get("/account/settings", account::get_settings_handler, "account/get_settings");
    router.post("/account/settings", account::post_settings_handler, "account/post_settings");
    router.post("/account/password", account::post_password_update, "account/post_password");
    router.post("/account/username", account::post_username_update, "account/post_username");
    router.post("/account/preference/menu", account::post_preference_menu, "account/post_preference_menu");
    router.post("/account/preference/theme", account::post_preference_theme, "account/post_preference_theme");

    router.get("/gist/new", gist::new_handler, "gist/new");
    router.post("/gist/create", gist::create_handler, "gist/create");
    router.get("/gist/list", gist::list_handler, "gist/list");
    router.get("/gist/show/:id", gist::show_handler, "gist/show");
    router.get("/gist/edit/:id", gist::edit_handler, "gist/edit");
    router.post("/gist/update", gist::update_handler, "gist/update");
    router.get("/gist/delete/:id", gist::delete_handler, "gist/delete");
    router.post("/gist/comment", gist::comment_handler, "gist/comment");
    router.post("/gist/comment/:id", gist::comment_update_handler, "gist/comment/update");

    router.get("/:kind/new", post::post::new_handler, "post/new");
    router.post("/:kind/create", post::post::create_handler, "post/create");
    router.get("/:kind/list", post::post::list_handler, "post/list");
    router.get("/:kind/show/:id", post::post::show_handler, "post/show/:id");
    router.get("/:kind/delete/:id", post::post::delete_handler, "post/delete");
    router.get("/:kind/edit/:id", post::post::edit_handler, "post/edit");
    router.post("/:kind/update", post::post::update_handler, "post/update");
    router.post("/:kind/comment", post::comment::comment_handler, "post/comment");
    router.post("/:kind/comment/:id", post::comment::comment_update_handler, "post/comment/update");
    router.post("/:kind/tags/:id", post::post::tags_update_handler, "post/tags/update");

    router.post("/:kind/stock/:id", post::stock::stock_handler, "post/stock");
    router.post("/:kind/unstock/:id", post::stock::unstock_handler, "post/unstock");
    router.post("/:kind/share/:id", post::share::share_handler, "post/share");

    router.get("/stocked/list", post::stock::stocked_list_handler, "stocked/list");
    router.get("/draft/list", post::draft::draft_list_handler, "draft/list");

    router.get("/tweet/list", tweet::list_handler, "tweet/list");
    router.post("/tweet/post", tweet::post_handler, "tweet/post");
    router.get("/tweet/show/:id", tweet::show_handler, "tweet/show");
    router.post("/tweet/comment", tweet::comment_handler, "tweet/comment");

    router.get("/tag/list", post::tag::tag_list_handler, "tag/list");

    router.get("/notifications", post::post::notifications_handler, "post/notifications");
    router.get("/notification_count", post::post::notification_count_handler, "post/notification_count");

    router.get("/:username", account::profile_post_handler, "user/profile");
    router.get("/:username/post", account::profile_post_handler, "user/profile_post");
    router.get("/:username/nippo", account::profile_nippo_handler, "user/profile_nippo");

    return router;
}

pub fn mount_path() -> Mount {
    let mut mount = Mount::new();
    mount.mount("/css", Static::new(Path::new("./public/css/")));
    mount.mount("/js", Static::new(Path::new("./public/js/")));
    mount.mount("/img", Static::new(Path::new("./public/img/")));
    mount.mount("/webfonts", Static::new(Path::new("./public/webfonts/")));
    mount.mount("/codemirror", Static::new(Path::new("./public/codemirror/")));
    mount.mount("/favicons", Static::new(Path::new("./public/favicons/")));
    mount.mount("/", create_router());

    return mount
}