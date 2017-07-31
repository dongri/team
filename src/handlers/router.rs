use Router;
use handlers::index;
use handlers::account;
use handlers::nippo;
use handlers::post;

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

    router.get("/account/settings", account::get_settings_handler, "account/get_settings");
    router.post("/account/settings", account::post_settings_handler, "account/post_settings");
    router.post("/account/password", account::post_password_update, "account/post_password");
    router.post("/account/username", account::post_username_update, "account/post_username");

    router.get("/nippo/new", nippo::new_handler, "nippo/new");
    router.post("/nippo/create", nippo::create_handler, "nippo/create");
    router.get("/nippo/list", nippo::list_handler, "nippo/list");
    router.get("/nippo/show/:id", nippo::show_handler, "nippo/show/:id");
    router.get("/nippo/delete/:id", nippo::delete_handler, "nippo/delete");
    router.get("/nippo/edit/:id", nippo::edit_handler, "nippo/edit");
    router.post("/nippo/update", nippo::update_handler, "nippo/update");
    router.post("/nippo/comment", nippo::comment_handler, "nippo/comment");
    router.post("/nippo/comment/:id", post::comment_update_handler, "nippo/comment/update");

    router.get("/post/new", post::new_handler, "post/new");
    router.post("/post/create", post::create_handler, "post/create");
    router.get("/post/list", post::list_handler, "post/list");
    router.get("/post/show/:id", post::show_handler, "post/show/:id");
    router.get("/post/delete/:id", post::delete_handler, "post/delete");
    router.get("/post/edit/:id", post::edit_handler, "post/edit");
    router.post("/post/update", post::update_handler, "post/update");
    router.post("/post/comment", post::comment_handler, "post/comment");
    router.post("/post/comment/:id", post::comment_update_handler, "post/comment/update");

    router.post("/post/stock/:id", post::stock_handler, "post/stock");
    router.post("/post/unstock/:id", post::unstock_handler, "post/unstock");

    router.get("/stocked/list", post::stocked_list_handler, "stocked/list");
    router.get("/draft/list", post::draft_list_handler, "draft/list");

    return router;
}
