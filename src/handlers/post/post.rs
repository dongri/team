use iron::prelude::*;
use iron::status;
use router::Router;
use hbs::Template;
use iron::modifiers::Redirect;
use hbs::handlebars::to_json;
use iron::Url;
use iron::prelude::IronResult;
use iron::mime::Mime;
use diff;
use db;
use persistent;

use env::CONFIG;
use helper;
use models;
use handlers;

pub const PAGINATES_PER: i32 = 10;
// const POST_KIND: &str = "post";

pub fn new_handler(req: &mut Request) -> IronResult<Response> {
    let conn = get_pg_connection!(req);
    let mut login_user: models::user::UserWithPreference = models::user::UserWithPreference{..Default::default()};
    match handlers::account::current_user(req, &conn) {
        Ok(user) => { login_user = user; }
        Err(e) => { error!("Errored: {:?}", e); }
    }
    let login_id = login_user.id;
    if login_id == 0 {
        return Ok(Response::with((status::Found, Redirect(helper::redirect_url("/signin")))));
    }

    let ref kind = req.extensions
        .get::<Router>()
        .unwrap()
        .find("kind")
        .unwrap_or("/");

    let mut resp = Response::new();

    #[derive(Serialize)]
    struct Data {
        logged_in: bool,
        login_user: models::user::UserWithPreference,
        kind: String,
        kind_title: String,
    }
    let data = Data {
        logged_in: login_id != 0,
        login_user: login_user,
        kind: kind.to_string(),
        kind_title: helper::uppercase_first_letter(kind),
    };
    resp.set_mut(helper::template("post/form", to_json(&data)))
        .set_mut(status::Ok);
    return Ok(resp);
}

pub fn create_handler(req: &mut Request) -> IronResult<Response> {
    let conn = get_pg_connection!(req);
    let mut login_user: models::user::UserWithPreference = models::user::UserWithPreference{..Default::default()};
    match handlers::account::current_user(req, &conn) {
        Ok(user) => { login_user = user; }
        Err(e) => { error!("Errored: {:?}", e); }
    }
    let login_id = login_user.id;
    if login_id == 0 {
        return Ok(Response::with((status::Found, Redirect(helper::redirect_url("/signin")))));
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
            _ => return Ok(Response::with(status::BadRequest)),
        }

        match map.get("title") {
            Some(&Value::String(ref name)) => {
                title = name.to_string();
            }
            _ => return Ok(Response::with(status::BadRequest)),
        }
        match map.get("body") {
            Some(&Value::String(ref name)) => {
                body = name.to_string();
            }
            _ => return Ok(Response::with(status::BadRequest)),
        }
        match map.get("tags") {
            Some(&Value::String(ref name)) => {
                tags = name.to_string();
            },
            _ => return Ok(Response::with(status::BadRequest)),
        }
    }

    let ref kind = req.extensions
        .get::<Router>()
        .unwrap()
        .find("kind")
        .unwrap_or("/");

    match models::post::create(&conn, kind, &login_id, &action, &title, &body, &tags) {
        Ok(id) => {
            let url_str = format!("{}/{}/show/{}", &CONFIG.team_domain, kind, id)
                         .to_string();

            if action == "publish" {
                let mut title = String::from("New post");
                let path = String::from("post");
                if kind == &"nippo" {
                    title = String::from("New 日報");
                }
                helper::post_to_slack(&conn, &login_id, &title, &body, &id, Vec::new(), &path);
                if kind == &"nippo" {
                    helper::webhook(login_user.username, title, body, url_str);
                }
            }
            let url = Url::parse(&format!("{}/{}/show/{}", &CONFIG.team_domain, kind, id)
                                     .to_string())
                    .unwrap();
            return Ok(Response::with((status::Found, Redirect(url))));
        }
        Err(e) => {
            error!("Errored: {:?}", e);
            return Ok(Response::with(status::InternalServerError));
        }
    }
}

pub fn list_handler(req: &mut Request) -> IronResult<Response> {
    let conn = get_pg_connection!(req);
    let mut login_user: models::user::UserWithPreference = models::user::UserWithPreference{..Default::default()};
    match handlers::account::current_user(req, &conn) {
        Ok(user) => { login_user = user; }
        Err(e) => { error!("Errored: {:?}", e); }
    }
    let login_id = login_user.id;
    if login_id == 0 {
        return Ok(Response::with((status::Found, Redirect(helper::redirect_url("/signin")))));
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

    let ref kind = req.extensions
        .get::<Router>()
        .unwrap()
        .find("kind")
        .unwrap_or("/");

    let mut resp = Response::new();

    #[derive(Serialize, Debug)]
    struct Data {
        logged_in: bool,
        login_user: models::user::UserWithPreference,
        posts: Vec<models::post::Post>,
        current_page: i32,
        total_page: i32,
        next_page: i32,
        prev_page: i32,
        kind: String,
        new_title: String,
        kind_title: String,
    }

    let mut page = page_param.parse::<i32>().unwrap();
    if page <= 0 {
        page = 1;
    }
    let offset = (page - 1) * PAGINATES_PER;
    let limit = PAGINATES_PER;

    let posts: Vec<models::post::Post>;
    let count: i32;

    match models::post::list(&conn, kind, &offset, &limit) {
        Ok(posts_db) => {
            posts = posts_db;
        }
        Err(e) => {
            error!("Errored: {:?}", e);
            return Ok(Response::with(status::InternalServerError));
        }
    }

    match models::post::count(&conn, kind) {
        Ok(count_db) => {
            count = count_db;
        }
        Err(e) => {
            error!("Errored: {:?}", e);
            return Ok(Response::with(status::InternalServerError));
        }
    }

    let new_title = if kind == &"post" {
        "Post Know-how"
    } else if kind == &"nippo" {
        "Post Nippo"
    } else {
        ""
    };

    let data = Data {
        logged_in: login_id != 0,
        login_user: login_user,
        posts: posts,
        current_page: page,
        total_page: count / PAGINATES_PER + 1,
        next_page: page + 1,
        prev_page: page - 1,
        kind: kind.to_string(),
        kind_title: helper::uppercase_first_letter(kind),
        new_title: new_title.to_string(),
    };

    resp.set_mut(Template::new("post/list", to_json(&data)))
        .set_mut(status::Ok);
    return Ok(resp);
}

pub fn show_handler(req: &mut Request) -> IronResult<Response> {
    let conn = get_pg_connection!(req);
    let mut login_user: models::user::UserWithPreference = models::user::UserWithPreference{..Default::default()};
    match handlers::account::current_user(req, &conn) {
        Ok(user) => { login_user = user; }
        Err(e) => { error!("Errored: {:?}", e); }
    }
    let login_id = login_user.id;
    if login_id == 0 {
        return Ok(Response::with((status::Found, Redirect(helper::redirect_url("/signin")))));
    }

    let mut resp = Response::new();

    let ref id_str = req.extensions
        .get::<Router>()
        .unwrap()
        .find("id")
        .unwrap_or("/");
    let id = id_str.parse::<i32>().unwrap();

    let ref kind = req.extensions
        .get::<Router>()
        .unwrap()
        .find("kind")
        .unwrap_or("/");

    #[derive(Serialize, Debug, Default)]
    struct PostComment {
        comment: models::post::Comment,
        editable: bool,
        kind: String,
    }

    #[derive(Serialize)]
    struct Data {
        logged_in: bool,
        login_user: models::user::UserWithPreference,
        post: models::post::Post,
        editable: bool,
        deletable: bool,
        shared: bool,
        comments: Vec<PostComment>,
        stocked: bool,
        kind: String,
        kind_title: String,
    }

    let post: models::post::Post;
    let comments: Vec<models::post::Comment>;
    let stocked: bool;

    match models::post::get_by_id(&conn, &id) {
        Ok(post_obj) => {
            post = post_obj;
        }
        Err(e) => {
            error!("Errored: {:?}", e);
            return Ok(Response::with(status::InternalServerError));
        }
    }

    match models::post::get_comments_by_post_id(&conn, &id) {
        Ok(comments_obj) => {
            comments = comments_obj;
        }
        Err(e) => {
            error!("Errored: {:?}", e);
            return Ok(Response::with(status::InternalServerError));
        }
    }

    match models::post::is_stocked(&conn, &login_id, &id) {
        Ok(is_stocked) => {
            stocked = is_stocked;
        }
        Err(e) => {
            error!("Errored: {:?}", e);
            return Ok(Response::with(status::InternalServerError));
        }
    }

    let mut post_comments: Vec<PostComment> = Vec::new();
    for comment in comments {
        let owner_id = comment.user_id;
        let pc = PostComment{
            comment: comment,
            editable: owner_id == login_id,
            kind: kind.to_string(),
        };
        post_comments.push(pc);
    }

    let shared = post.shared;
    let owner_id = post.user_id;
    let deletable = owner_id == login_id || post.shared;
    let editable = owner_id == login_id || post.shared;
    let data = Data {
        logged_in: login_id != 0,
        login_user: login_user,
        post: post,
        editable: editable,
        deletable: deletable,
        shared: shared,
        comments: post_comments,
        stocked: stocked,
        kind: kind.to_string(),
        kind_title: helper::uppercase_first_letter(kind),
    };

    resp.set_mut(Template::new("post/show", to_json(&data)))
        .set_mut(status::Ok);
    return Ok(resp);
}

pub fn delete_handler(req: &mut Request) -> IronResult<Response> {
    let conn = get_pg_connection!(req);
    let mut login_user: models::user::UserWithPreference = models::user::UserWithPreference{..Default::default()};
    match handlers::account::current_user(req, &conn) {
        Ok(user) => { login_user = user; }
        Err(e) => { error!("Errored: {:?}", e); }
    }
    if login_user.id == 0 {
        return Ok(Response::with((status::Found, Redirect(helper::redirect_url("/signin")))));
    }

    let ref id_str = req.extensions
        .get::<Router>()
        .unwrap()
        .find("id")
        .unwrap_or("/");
    let id = id_str.parse::<i32>().unwrap();

    match models::post::get_by_id(&conn, &id) {
        Ok(post) => {
            if post.user_id != login_user.id {
                return Ok(Response::with(status::Forbidden));
            }
        }
        Err(e) => {
            error!("Errored: {:?}", e);
            return Ok(Response::with(status::InternalServerError));
        }
    }

    match models::post::delete_by_id(&conn, &id) {
        Ok(_) => {
            return Ok(Response::with((status::Found, Redirect(helper::redirect_url("/post/list")))));
        }
        Err(e) => {
            error!("Errored: {:?}", e);
            Ok(Response::with(status::InternalServerError))
        }
    }
}

pub fn edit_handler(req: &mut Request) -> IronResult<Response> {
    let conn = get_pg_connection!(req);
    let mut login_user: models::user::UserWithPreference = models::user::UserWithPreference{..Default::default()};
    match handlers::account::current_user(req, &conn) {
        Ok(user) => { login_user = user; }
        Err(e) => { error!("Errored: {:?}", e); }
    }
    let login_id = login_user.id;
    if login_id == 0 {
        return Ok(Response::with((status::Found, Redirect(helper::redirect_url("/signin")))));
    }

    let mut resp = Response::new();
    #[derive(Serialize)]
    struct Data {
        logged_in: bool,
        login_user: models::user::UserWithPreference,
        post: models::post::Post,
        tags: String,
        kind: String,
        kind_title: String,
    }

    let post: models::post::Post;

    let ref id_str = req.extensions
        .get::<Router>()
        .unwrap()
        .find("id")
        .unwrap_or("/");
    let id = id_str.parse::<i32>().unwrap();

    let ref kind = req.extensions
        .get::<Router>()
        .unwrap()
        .find("kind")
        .unwrap_or("/");

    match models::post::get_by_id(&conn, &id) {
        Ok(post_obj) => {
            if post_obj.user_id != login_id && post_obj.shared == false {
                return Ok(Response::with(status::Forbidden));
            }
            post = post_obj;
        }
        Err(e) => {
            error!("Errored: {:?}", e);
            return Ok(Response::with(status::InternalServerError));
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
        kind: kind.to_string(),
        kind_title: helper::uppercase_first_letter(kind),
    };
    resp.set_mut(Template::new("post/edit", to_json(&data)))
        .set_mut(status::Ok);
    return Ok(resp);
}

pub fn update_handler(req: &mut Request) -> IronResult<Response> {
    let conn = get_pg_connection!(req);
    let mut login_user: models::user::UserWithPreference = models::user::UserWithPreference{..Default::default()};
    match handlers::account::current_user(req, &conn) {
        Ok(user) => { login_user = user; }
        Err(e) => { error!("Errored: {:?}", e); }
    }
    let login_id = login_user.id;
    if login_id == 0 {
        return Ok(Response::with((status::Found, Redirect(helper::redirect_url("/signin")))));
    }

    use params::{Params, Value};

    let id: i32;
    let title: String;
    let body: String;
    let tags: String;
    let action: String;

    let old_post: models::post::Post;
    {
        let map = req.get_ref::<Params>().unwrap();
        match map.find(&["id"]) {
            Some(&Value::String(ref name)) => {
                id = name.to_string().parse::<i32>().unwrap();
            }
            _ => return Ok(Response::with(status::BadRequest)),
        }

        match map.find(&["title"]) {
            Some(&Value::String(ref name)) => {
                title = name.to_string();
            }
            _ => return Ok(Response::with(status::BadRequest)),
        }

        match map.find(&["body"]) {
            Some(&Value::String(ref name)) => {
                body = name.to_string();
            }
            _ => return Ok(Response::with(status::BadRequest)),
        }

        match map.find(&["tags"]) {
            Some(&Value::String(ref name)) => {
                tags = name.to_string();
            },
            _ => return Ok(Response::with(status::BadRequest)),
        }

        match map.find(&["action"]) {
            Some(&Value::String(ref name)) => {
                action = name.to_string();
            },
            _ => return Ok(Response::with(status::BadRequest)),
        }
    }

    let ref kind = req.extensions
        .get::<Router>()
        .unwrap()
        .find("kind")
        .unwrap_or("/");

    match models::post::get_by_id(&conn, &id) {
        Ok(post_obj) => {
            old_post = post_obj;
            if old_post.user_id != login_id && old_post.shared == false {
                return Ok(Response::with(status::Forbidden));
            }
        }
        Err(e) => {
            error!("Errored: {:?}", e);
            return Ok(Response::with(status::InternalServerError));
        }
    }

    match models::post::update(&conn, &id, &title, &body, &tags, &action) {
        Ok(_) => {
            let title = String::from("Edit post");
            let path = String::from("post");
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
                if old_post.status == "draft" {
                    diff_body = body.clone();
                }
                helper::post_to_slack(&conn, &login_id, &title, &diff_body, &id, Vec::new(), &path);
                if kind == &"nippo" && old_post.status == "draft" {
                    let title = String::from("New 日報");
                    let webhook_body = body.clone();
                    let url_str = format!("{}/{}/show/{}", &CONFIG.team_domain, kind, id).to_string();
                    helper::webhook(login_user.username, title, webhook_body, url_str);
                }
            }

            let url = Url::parse(&format!("{}/{}/show/{}", &CONFIG.team_domain, kind, id)
                                     .to_string())
                    .unwrap();
            return Ok(Response::with((status::Found, Redirect(url))));
        }
        Err(e) => {
            error!("Errored: {:?}", e);
            return Ok(Response::with(status::InternalServerError));
        }
    }
}

pub fn tags_update_handler(req: &mut Request) -> IronResult<Response> {
    let conn = get_pg_connection!(req);
    let mut login_user: models::user::UserWithPreference = models::user::UserWithPreference{..Default::default()};
    match handlers::account::current_user(req, &conn) {
        Ok(user) => { login_user = user; }
        Err(e) => { error!("Errored: {:?}", e); }
    }
    let login_id = login_user.id;
    if login_id == 0 {
        return Ok(Response::with((status::Found, Redirect(helper::redirect_url("/signin")))));
    }

    use params::{Params, Value};

    let id: i32;
    let title: String;
    let body: String;
    let tags: String;
    let action: String = String::from("publish");

    let old_post: models::post::Post;
    {
        let map = req.get_ref::<Params>().unwrap();
        match map.find(&["id"]) {
            Some(&Value::String(ref name)) => {
                id = name.to_string().parse::<i32>().unwrap();
            }
            _ => return Ok(Response::with(status::BadRequest)),
        }

        match map.find(&["tags"]) {
            Some(&Value::String(ref name)) => {
                tags = name.to_string();
            },
            _ => return Ok(Response::with(status::BadRequest)),
        }
    }

    let ref kind = req.extensions
        .get::<Router>()
        .unwrap()
        .find("kind")
        .unwrap_or("/");

    match models::post::get_by_id(&conn, &id) {
        Ok(post_obj) => {
            old_post = post_obj;
            title = old_post.title.clone();
            body = old_post.body.clone();
        }
        Err(e) => {
            error!("Errored: {:?}", e);
            return Ok(Response::with(status::InternalServerError));
        }
    }

    match models::post::update(&conn, &id, &title, &body, &tags, &action) {
        Ok(_) => {
            let title = String::from("Update tag");
            let path = String::from("post");
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
            helper::post_to_slack(&conn, &login_id, &title, &diff_body, &id, Vec::new(), &path);

            let url = Url::parse(&format!("{}/{}/show/{}", &CONFIG.team_domain, kind, id)
                                     .to_string())
                    .unwrap();
            return Ok(Response::with((status::Found, Redirect(url))));
        }
        Err(e) => {
            error!("Errored: {:?}", e);
            return Ok(Response::with(status::InternalServerError));
        }
    }
}

pub fn notifications_handler(req: &mut Request) -> IronResult<Response> {
    let conn = get_pg_connection!(req);
    let mut login_user: models::user::UserWithPreference = models::user::UserWithPreference{..Default::default()};
    match handlers::account::current_user(req, &conn) {
        Ok(user) => { login_user = user; }
        Err(e) => { error!("Errored: {:?}", e); }
    }
    let login_id = login_user.id;
    if login_id == 0 {
        return Ok(Response::with((status::Found, Redirect(helper::redirect_url("/signin")))));
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
        login_user: models::user::UserWithPreference,
        notifications: Vec<models::notification::Notification>,
        current_page: i32,
        total_page: i32,
        next_page: i32,
        prev_page: i32,
    }

    let mut page = page_param.parse::<i32>().unwrap();
    let offset = (page - 1) * PAGINATES_PER;
    let limit = PAGINATES_PER;

    let notifications: Vec<models::notification::Notification>;
    let count: i32;

    match models::notification::list(&conn, &login_id, &offset, &limit) {
        Ok(notifications_db) => {
            notifications = notifications_db;
        }
        Err(e) => {
            error!("Errored: {:?}", e);
            return Ok(Response::with(status::InternalServerError));
        }
    }

    match models::notification::count(&conn, &login_id) {
        Ok(count_db) => {
            count = count_db;
        }
        Err(e) => {
            error!("Errored: {:?}", e);
            return Ok(Response::with(status::InternalServerError));
        }
    }

    if page == 0 {
        page = 1;
    }

    let data = Data {
        logged_in: login_id != 0,
        login_user: login_user,
        notifications: notifications,
        current_page: page,
        total_page: count / PAGINATES_PER + 1,
        next_page: page + 1,
        prev_page: page - 1,
    };

    resp.set_mut(Template::new("notifications", to_json(&data)))
        .set_mut(status::Ok);
    return Ok(resp);
}

pub fn notification_count_handler(req: &mut Request) -> IronResult<Response> {
    let conn = get_pg_connection!(req);
    let mut login_user: models::user::UserWithPreference = models::user::UserWithPreference{..Default::default()};
    match handlers::account::current_user(req, &conn) {
        Ok(user) => { login_user = user; }
        Err(e) => { error!("Errored: {:?}", e); }
    }
    let login_id = login_user.id;
    if login_id == 0 {
        return Ok(Response::with((status::Found, Redirect(helper::redirect_url("/signin")))));
    }
    #[derive(Serialize, Debug)]
    struct Data {
        count: i32,
    }
    let count: i32;
    match models::notification::unread_count(&conn, &login_id) {
        Ok(count_db) => {
            count = count_db;
        }
        Err(e) => {
            error!("Errored: {:?}", e);
            return Ok(Response::with(status::InternalServerError));
        }
    }
    let data = Data {
        count: count,
    };
    let content_type = "application/json".parse::<Mime>().unwrap();
    return Ok(Response::with((content_type, status::Ok, to_json(&data).to_string())));
}

use time;
use std::fs;
pub fn image_upload_handler(req: &mut Request) -> IronResult<Response> {
    let conn = get_pg_connection!(req);
    let mut login_user: models::user::UserWithPreference = models::user::UserWithPreference{..Default::default()};
    match handlers::account::current_user(req, &conn) {
        Ok(user) => { login_user = user; }
        Err(e) => { error!("Errored: {:?}", e); }
    }
    let login_id = login_user.id;
    if login_id == 0 {
        return Ok(Response::with((status::Found, Redirect(helper::redirect_url("/signin")))));
    }

    use params::{Params, Value};

    let timestamp = time::get_time().sec;
    let filepath = format!("public/img/posts/{:?}.png", timestamp);
    let fileurl = format!("img/posts/{:?}.png", timestamp);

    match req.get_ref::<Params>().unwrap().find(&["file"]) {
        Some(&Value::File(ref file)) => {
            let a = &file.path;
            let b = &filepath;
            let _ = fs::copy(a, b);
        }
        _ => {
            println!("no file");
        }
    }
    #[derive(Serialize, Debug)]
    struct Data {
        fileurl: String,
    }
    let data = Data {
        fileurl: format!("{}/{}", &CONFIG.team_domain, fileurl),
    };
    let content_type = "application/json".parse::<Mime>().unwrap();
    return Ok(Response::with((content_type, status::Ok, to_json(&data).to_string())));
}