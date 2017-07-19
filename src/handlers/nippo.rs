use persistent;
use iron::prelude::*;
use iron::status;
use router::Router;
use hbs::Template;
use iron::modifiers::Redirect;
use hbs::handlebars::to_json;
use iron::Url;
use iron::prelude::IronResult;
use diff;

use db;
use helper;
use env::CONFIG;
use models;
use handlers;

const PAGINATES_PER: i32 = 10;
const POST_KIND: &str = "nippo";

pub fn new_handler(req: &mut Request) -> IronResult<Response> {
    let conn = get_pg_connection!(req);
    let mut login_user: models::user::User = models::user::User{..Default::default()};
    match handlers::account::current_user(req, &conn) {
        Ok(user) => { login_user = user; }
        Err(e) => { error!("Errored: {:?}", e); }
    }
    let login_id = login_user.id;
    if login_id == 0 {
        return Ok(Response::with((status::Found, Redirect(url_for!(req, "account/get_signin")))));
    }

    let mut resp = Response::new();

    #[derive(Serialize)]
    struct Data {
        logged_in: bool,
        login_user: models::user::User,
    }
    let data = Data {
        logged_in: login_id != 0,
        login_user: login_user,
    };
    resp.set_mut(helper::template("nippo/form", to_json(&data)))
        .set_mut(status::Ok);
    return Ok(resp);
}

pub fn create_handler(req: &mut Request) -> IronResult<Response> {
    let conn = get_pg_connection!(req);
    let mut login_user: models::user::User = models::user::User{..Default::default()};
    match handlers::account::current_user(req, &conn) {
        Ok(user) => { login_user = user; }
        Err(e) => { error!("Errored: {:?}", e); }
    }
    let login_id = login_user.id;
    if login_id == 0 {
        return Ok(Response::with((status::Found, Redirect(url_for!(req, "account/get_signin")))));
    }

    let action: String;
    let title: String;
    let body: String;
    let tags: String;

    {
        use params::{Params, Value};
        let map = req.get_ref::<Params>().unwrap();

        match map.get("action") {
            Some(&Value::String(ref name)) => {
                action = name.to_string();
            }
            _ => return Ok(Response::with((status::BadRequest))),
        }

        match map.get("title") {
            Some(&Value::String(ref name)) => {
                title = name.to_string();
            }
            _ => return Ok(Response::with((status::BadRequest))),
        }
        match map.get("body") {
            Some(&Value::String(ref name)) => {
                body = name.to_string();
            }
            _ => return Ok(Response::with((status::BadRequest))),
        }
        match map.get("tags") {
            Some(&Value::String(ref name)) => {
                tags = name.to_string();
            },
            _ => return Ok(Response::with((status::BadRequest))),
        }
    }

    match models::post::create(&conn, POST_KIND, &login_id, &action, &title, &body, &tags) {
        Ok(id) => {
            let url_str = format!("{}/{}/{}", &CONFIG.team_domain, "nippo/show", id)
                                     .to_string();
            let url = Url::parse(&url_str).unwrap();

            if action == "publish" {
                let slack_title = String::from("New nippo");
                helper::post_to_slack(&conn, &login_id, &slack_title, &body, &id);

                helper::webhook(login_user.username, title, body, url_str);
            }

            return Ok(Response::with((status::Found, Redirect(url))));
        }
        Err(e) => {
            error!("Errored: {:?}", e);
            return Ok(Response::with((status::InternalServerError)));
        }
    }
}

pub fn list_handler(req: &mut Request) -> IronResult<Response> {
    let conn = get_pg_connection!(req);
    let mut login_user: models::user::User = models::user::User{..Default::default()};
    match handlers::account::current_user(req, &conn) {
        Ok(user) => { login_user = user; }
        Err(e) => { error!("Errored: {:?}", e); }
    }
    let login_id = login_user.id;
    if login_id == 0 {
        return Ok(Response::with((status::Found, Redirect(url_for!(req, "account/get_signin")))));
    }

    let page_param: String;

    {
        use params::{Params, Value};
        let map = req.get_ref::<Params>().unwrap();
        match map.get("page") {
            Some(&Value::String(ref name)) => {
                page_param = name.to_string();
            }
            _ => page_param = "1".to_string(),
        }
    }

    let mut resp = Response::new();

    #[derive(Serialize, Debug)]
    struct Data {
        logged_in: bool,
        login_user: models::user::User,
        posts: Vec<models::post::Post>,
        current_page: i32,
        total_page: i32,
        next_page: i32,
        prev_page: i32,
    }

    let mut page = page_param.parse::<i32>().unwrap();
    let offset = (page - 1) * PAGINATES_PER;
    let limit = PAGINATES_PER;

    let posts: Vec<models::post::Post>;
    let count: i32;

    match models::post::list(&conn, POST_KIND, &offset, &limit) {
        Ok(posts_db) => {
            posts = posts_db;
        }
        Err(e) => {
            error!("Errored: {:?}", e);
            return Ok(Response::with((status::InternalServerError)));
        }
    }

    match models::post::count(&conn, POST_KIND) {
        Ok(count_db) => {
            count = count_db;
        }
        Err(e) => {
            error!("Errored: {:?}", e);
            return Ok(Response::with((status::InternalServerError)));
        }
    }

    if page == 0 {
        page = 1;
    }
    let data = Data {
        logged_in: login_id != 0,
        login_user: login_user,
        posts: posts,
        current_page: page,
        total_page: count / PAGINATES_PER + 1,
        next_page: page + 1,
        prev_page: page - 1,
    };

    resp.set_mut(Template::new("nippo/list", to_json(&data)))
        .set_mut(status::Ok);
    return Ok(resp);
}

pub fn show_handler(req: &mut Request) -> IronResult<Response> {
    let conn = get_pg_connection!(req);
    let mut login_user: models::user::User = models::user::User{..Default::default()};
    match handlers::account::current_user(req, &conn) {
        Ok(user) => { login_user = user; }
        Err(e) => { error!("Errored: {:?}", e); }
    }
    let login_id = login_user.id;
    if login_id == 0 {
        return Ok(Response::with((status::Found, Redirect(url_for!(req, "account/get_signin")))));
    }

    let mut resp = Response::new();

    let ref id_str = req.extensions
        .get::<Router>()
        .unwrap()
        .find("id")
        .unwrap_or("/");
    let id = id_str.parse::<i32>().unwrap();

    #[derive(Serialize)]
    struct Data {
        logged_in: bool,
        login_user: models::user::User,
        post: models::post::Post,
        editable: bool,
        comments: Vec<models::post::Comment>,
    }

    let post: models::post::Post;
    let comments: Vec<models::post::Comment>;

    match models::post::get_by_id(&conn, &id) {
        Ok(post_db) => {
            post = post_db;
        }
        Err(e) => {
            error!("Errored: {:?}", e);
            return Ok(Response::with((status::InternalServerError)));
        }
    }

    match models::post::get_comments_by_post_id(&conn, &id) {
        Ok(comments_db) => {
            comments = comments_db;
        }
        Err(e) => {
            error!("Errored: {:?}", e);
            return Ok(Response::with((status::InternalServerError)));
        }
    }

    let owner_id = post.user_id;
    let data = Data {
        logged_in: login_id != 0,
        login_user: login_user,
        post: post,
        editable: owner_id == login_id,
        comments: comments,
    };

    resp.set_mut(Template::new("nippo/show", to_json(&data)))
        .set_mut(status::Ok);
    return Ok(resp);
}

pub fn delete_handler(req: &mut Request) -> IronResult<Response> {
    let conn = get_pg_connection!(req);
    let mut login_user: models::user::User = models::user::User{..Default::default()};
    match handlers::account::current_user(req, &conn) {
        Ok(user) => { login_user = user; }
        Err(e) => { error!("Errored: {:?}", e); }
    }
    let login_id = login_user.id;
    if login_id == 0 {
        return Ok(Response::with((status::Found, Redirect(url_for!(req, "account/get_signin")))));
    }

    let ref id_str = req.extensions
        .get::<Router>()
        .unwrap()
        .find("id")
        .unwrap_or("/");
    let id = id_str.parse::<i32>().unwrap();

    match models::post::get_by_id(&conn, &id) {
        Ok(nippo) => {
            if nippo.user_id != login_id {
                return Ok(Response::with((status::Forbidden)));
            }
        }
        Err(e) => {
            error!("Errored: {:?}", e);
            return Ok(Response::with((status::InternalServerError)));
        }
    }

    match models::post::delete_by_id(&conn, &id) {
        Ok(_) => {
            return Ok(Response::with((status::Found, Redirect(url_for!(req, "nippo/list")))));
        }
        Err(e) => {
            error!("Errored: {:?}", e);
            Ok(Response::with((status::InternalServerError)))
        }
    }
}

pub fn edit_handler(req: &mut Request) -> IronResult<Response> {
    let conn = get_pg_connection!(req);
    let mut login_user: models::user::User = models::user::User{..Default::default()};
    match handlers::account::current_user(req, &conn) {
        Ok(user) => { login_user = user; }
        Err(e) => { error!("Errored: {:?}", e); }
    }
    let login_id = login_user.id;
    if login_id == 0 {
        return Ok(Response::with((status::Found, Redirect(url_for!(req, "account/get_signin")))));
    }

    let mut resp = Response::new();
    #[derive(Serialize)]
    struct Data {
        logged_in: bool,
        login_user: models::user::User,
        post: models::post::Post,
        tags: String,
    }

    let post: models::post::Post;

    let ref id_str = req.extensions
        .get::<Router>()
        .unwrap()
        .find("id")
        .unwrap_or("/");
    let id = id_str.parse::<i32>().unwrap();

    match models::post::get_by_id(&conn, &id) {
        Ok(post_db) => {
            if post_db.user_id != login_id {
                return Ok(Response::with((status::Forbidden)));
            }
            post = post_db;
        }
        Err(e) => {
            error!("Errored: {:?}", e);
            return Ok(Response::with((status::InternalServerError)));
        }
    }

    let mut tag_str: String = String::from("");
    for t in &post.tags {
        tag_str = format!("{},{}", tag_str, t.name).to_string();
    }
    if tag_str.len() > 0 {
        tag_str.remove(0);
    }

    let data = Data {
        logged_in: login_id != 0,
        login_user: login_user,
        post: post,
        tags: tag_str,
    };
    resp.set_mut(Template::new("nippo/edit", to_json(&data)))
        .set_mut(status::Ok);
    return Ok(resp);
}

pub fn update_handler(req: &mut Request) -> IronResult<Response> {
    let conn = get_pg_connection!(req);
    let mut login_user: models::user::User = models::user::User{..Default::default()};
    match handlers::account::current_user(req, &conn) {
        Ok(user) => { login_user = user; }
        Err(e) => { error!("Errored: {:?}", e); }
    }
    let login_id = login_user.id;
    if login_id == 0 {
        return Ok(Response::with((status::Found, Redirect(url_for!(req, "account/get_signin")))));
    }

    let id: i32;
    let title: String;
    let body: String;
    let tags: String;
    let action: String;

    let old_post: models::post::Post;

    use params::{Params, Value};
    let map = req.get_ref::<Params>().unwrap();

    match map.find(&["id"]) {
        Some(&Value::String(ref name)) => {
            id = name.to_string().parse::<i32>().unwrap();
        }
        _ => return Ok(Response::with((status::BadRequest))),
    }

    match map.find(&["title"]) {
        Some(&Value::String(ref name)) => {
            title = name.to_string();
        }
        _ => return Ok(Response::with((status::BadRequest))),
    }

    match map.find(&["body"]) {
        Some(&Value::String(ref name)) => {
            body = name.to_string();
        }
        _ => return Ok(Response::with((status::BadRequest))),
    }

    match map.find(&["tags"]) {
        Some(&Value::String(ref name)) => {
            tags = name.to_string();
        },
        _ => return Ok(Response::with((status::BadRequest))),
    }

    match map.find(&["action"]) {
        Some(&Value::String(ref name)) => {
            action = name.to_string();
        }
        _ => return Ok(Response::with((status::BadRequest))),
    }


    match models::post::get_by_id(&conn, &id) {
        Ok(post_db) => {
            old_post = post_db;
            if old_post.user_id != login_id {
                return Ok(Response::with((status::Forbidden)));
            }
        }
        Err(e) => {
            error!("Errored: {:?}", e);
            return Ok(Response::with((status::InternalServerError)));
        }
    }

    match models::post::update(&conn, &id, &title, &body, &tags, &action) {
        Ok(_) => {
            let title = String::from("Edit nippo");
            let left = &old_post.body;
            let right = &body;
            let mut diff_body = String::from("");
            for diff in diff::lines(left, right) {
                match diff {
                    diff::Result::Left(l)    => diff_body += &format!("-{}\n", l),
                    diff::Result::Both(l, _) => debug!(" {}\n", l),
                    diff::Result::Right(r)   => diff_body += &format!("+{}\n", r)
                }
            }
            if action == "publish" {
                helper::post_to_slack(&conn, &login_id, &title, &diff_body, &id);
            }

            let url = Url::parse(&format!("{}/{}/{}", CONFIG.team_domain, "nippo/show", id)
                                     .to_string())
                    .unwrap();
            return Ok(Response::with((status::Found, Redirect(url))));
        }
        Err(e) => {
            error!("Errored: {:?}", e);
            return Ok(Response::with((status::InternalServerError)));
        }
    }
}

pub fn comment_handler(req: &mut Request) -> IronResult<Response> {
    let conn = get_pg_connection!(req);
    let mut login_user: models::user::User = models::user::User{..Default::default()};
    match handlers::account::current_user(req, &conn) {
        Ok(user) => { login_user = user; }
        Err(e) => { error!("Errored: {:?}", e); }
    }
    let login_id = login_user.id;
    if login_id == 0 {
        return Ok(Response::with((status::Found, Redirect(url_for!(req, "account/get_signin")))));
    }

    use params::{Params, Value};
    let map = req.get_ref::<Params>().unwrap();

    let id: i32;
    let body: String;

    match map.find(&["id"]) {
        Some(&Value::String(ref name)) => {
            id = name.parse::<i32>().unwrap();
        }
        _ => return Ok(Response::with((status::BadRequest))),
    }

    match map.find(&["body"]) {
        Some(&Value::String(ref name)) => {
            body = name.to_string();
        }
        _ => return Ok(Response::with((status::BadRequest))),
    }

    match models::post::add_comment(&conn, &login_id, &id, &body) {
        Ok(_) => {
            let title = String::from("New comment");
            helper::post_to_slack(&conn, &login_id, &title, &body, &id);

            let url = Url::parse(&format!("{}/{}/{}", &CONFIG.team_domain, "nippo/show", id)
                                     .to_string())
                    .unwrap();
            return Ok(Response::with((status::Found, Redirect(url))));
        }
        Err(e) => {
            error!("Errored: {:?}", e);
            return Ok(Response::with((status::InternalServerError)));
        }
    }
}
