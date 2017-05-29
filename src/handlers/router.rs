use Router;
use handlers::index;
use handlers::account;
use handlers::nippo;

pub fn create_router() -> Router {
    let mut router = Router::new();
    router.get("/", index::index_handler, "index");

    router.get("/signup", account::get_signup_handler, "account/get_signup");
    router.post("/signup", account::post_signup_handler, "account/post_signup");
    router.get("/signin", account::get_signin_handler, "account/get_signin");
    router.post("/signin", account::post_signin_handler, "account/post_signin");
    router.get("/signout", account::get_signout_handler, "account/get_signout");

    router.get("/nippo/new", nippo::new_handler, "nippo/new");
    router.post("/nippo/create", nippo::create_handler, "nippo/create");
    router.get("/nippo/list", nippo::list_handler, "nippo/list");
    router.get("/nippo/show/:id", nippo::show_handler, "nippo/show/:id");
    router.get("/nippo/delete/:id", nippo::delete_handler, "nippo/delete");
    router.get("/nippo/edit/:id", nippo::edit_handler, "nippo/edit");
    router.post("/nippo/update", nippo::update_handler, "nippo/update");
    router.post("/nippo/comment", nippo::comment_handler, "nippo/comment");
    return router;
}
