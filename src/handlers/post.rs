use persistent;
use iron::prelude::*;
use iron::status;
use router::{Router};
use hbs::{Template};
use iron::modifiers::{Redirect};
use hbs::handlebars::to_json;
use iron::{Url};
use iron::prelude::{IronResult};

use db;
use helper;
use models;
use handlers;

const PAGINATES_PER: i32 = 10;
const POST_KIND: &str = "post";

pub fn new_handler(req: &mut Request) -> IronResult<Response> {
    let login_id = handlers::account::get_login_id(req);
    if login_id == 0 {
        return Ok(Response::with((status::Found, Redirect(url_for!(req, "account/get_signin")))));
    }
    let mut resp = Response::new();

    #[derive(Serialize)]
    struct Data {
        logged_in: bool,
    }
    let data = Data {
        logged_in: login_id != 0,
    };
    resp.set_mut(helper::template("post/form", to_json(&data))).set_mut(status::Ok);
    return Ok(resp);
}

pub fn create_handler(req: &mut Request) -> IronResult<Response> {
    let login_id = handlers::account::get_login_id(req);
    if login_id == 0 {
        return Ok(Response::with((status::Found, Redirect(url_for!(req, "account/get_signin")))));
    }

    let conn = get_pg_connection!(req);

    let title: String;
    let body: String;
    let tags: String;

    {
        use params::{Params, Value};
        let map = req.get_ref::<Params>().unwrap();
        match map.get("title") {
            Some(&Value::String(ref name)) => {
                title = name.to_string();
            },
            _ => return Ok(Response::with((status::BadRequest))),
        }
        match map.get("body") {
            Some(&Value::String(ref name)) => {
                body = name.to_string();
            },
            _ => return Ok(Response::with((status::BadRequest))),
        }
        match map.get("tags") {
            Some(&Value::String(ref name)) => {
                tags = name.to_string();
            },
            _ => return Ok(Response::with((status::BadRequest))),
        }
    }
    let body_db = body.clone();
    let body_slack = body.clone();

    match models::post::create(conn, POST_KIND, login_id, title, body_db, tags) {
        Ok(id) => {
            let link = format!("{}/{}/{}", helper::get_domain(), "post/show", id).to_string();
            let text = format!("{}\n{}\n{}", "New post", body_slack, link).to_string();
            helper::slack(text);
            let url = Url::parse(&format!("{}/{}/{}", helper::get_domain(), "post/show", id).to_string()).unwrap();
            return Ok(Response::with((status::Found, Redirect(url))));
        },
        Err(e) => {
            println!("Errored: {:?}", e);
            return Ok(Response::with((status::InternalServerError)))
        }
    }
}

pub fn list_handler(req: &mut Request) -> IronResult<Response> {
    let login_id = handlers::account::get_login_id(req);
    if login_id == 0 {
        return Ok(Response::with((status::Found, Redirect(url_for!(req, "account/get_signin")))));
    }
    let conn_l = get_pg_connection!(req);
    let conn_c = get_pg_connection!(req);

    let page_param: String;

    {
        use params::{Params, Value};
        let map = req.get_ref::<Params>().unwrap();
        match map.get("page") {
            Some(&Value::String(ref name)) => {
                page_param = name.to_string();
            },
            _ => page_param = "1".to_string(),
        }
    }

    let mut resp = Response::new();

    #[derive(Serialize, Debug)]
    struct Data {
        logged_in: bool,
        posts: Vec<models::post::Post>,
        current_page: i32,
        total_page: i32,
        next_page: i32,
        prev_page: i32,
    }

    let mut page = page_param.parse::<i32>().unwrap();
    let offset = ( page - 1 ) * PAGINATES_PER;
    let limit = PAGINATES_PER;

    let posts: Vec<models::post::Post>;
    let count: i32;

    match models::post::list(conn_l, POST_KIND, offset, limit) {
        Ok(posts_db) => {
            posts = posts_db;
        },
        Err(e) => {
            println!("Errored: {:?}", e);
            return Ok(Response::with((status::InternalServerError)));
        }
    }

    match models::post::count(conn_c, POST_KIND) {
        Ok(count_db) => {
            count = count_db;
        },
        Err(e) => {
            println!("Errored: {:?}", e);
            return Ok(Response::with((status::InternalServerError)));
        }
    }

    if page == 0 {
        page = 1;
    }
    let data = Data {
        logged_in: login_id != 0,
        posts: posts,
        current_page: page,
        total_page: count / PAGINATES_PER + 1,
        next_page: page + 1,
        prev_page: page - 1,
    };

    resp.set_mut(Template::new("post/list", to_json(&data))).set_mut(status::Ok);
    return Ok(resp);
}

pub fn show_handler(req: &mut Request) -> IronResult<Response> {
    let login_id = handlers::account::get_login_id(req);
    if login_id == 0 {
        return Ok(Response::with((status::Found, Redirect(url_for!(req, "account/get_signin")))));
    }
    let conn_s = get_pg_connection!(req);
    let conn_c = get_pg_connection!(req);
    let mut resp = Response::new();

    let ref id_str = req.extensions.get::<Router>().unwrap().find("id").unwrap_or("/");
    let id = id_str.parse::<i32>().unwrap();

    #[derive(Serialize, Default)]
    struct Data {
        logged_in: bool,
        post: models::post::Post,
        editable: bool,
        comments: Vec<models::post::Comment>,
    }

    let post: models::post::Post;
    let comments: Vec<models::post::Comment>;

    match models::post::get_by_id(conn_s, id) {
        Ok(post_obj) => {
            post = post_obj;
        },
        Err(e) => {
            println!("Errored: {:?}", e);
            return Ok(Response::with((status::InternalServerError)))
        }
    }

    match models::post::get_comments_by_post_id(conn_c, id) {
        Ok(comments_obj) => {
            comments = comments_obj;
        },
        Err(e) => {
            println!("Errored: {:?}", e);
            return Ok(Response::with((status::InternalServerError)))
        }
    }

    let owner_id = post.user_id;
    let data = Data {
        logged_in: login_id != 0,
        post: post,
        editable: owner_id == login_id,
        comments: comments,
    };

    resp.set_mut(Template::new("post/show", to_json(&data))).set_mut(status::Ok);
    return Ok(resp);
}

pub fn delete_handler(req: &mut Request) -> IronResult<Response> {
    let login_id = handlers::account::get_login_id(req);
    if login_id == 0 {
        return Ok(Response::with((status::Found, Redirect(url_for!(req, "account/get_signin")))));
    }
    let conn_s = get_pg_connection!(req);
    let conn_d = get_pg_connection!(req);

    let ref id_str = req.extensions.get::<Router>().unwrap().find("id").unwrap_or("/");
    let id = id_str.parse::<i32>().unwrap();

    match models::post::get_by_id(conn_s, id) {
        Ok(post) => {
            if post.user_id != login_id {
                return Ok(Response::with((status::Forbidden)));
            }
        },
        Err(e) => {
            println!("Errored: {:?}", e);
            return Ok(Response::with((status::InternalServerError)));
        }
    }

    match models::post::delete_by_id(conn_d, id) {
        Ok(_) => {
            return Ok(Response::with((status::Found, Redirect(url_for!(req, "post/list")))));
        },
        Err(e) => {
            println!("Errored: {:?}", e);
            Ok(Response::with((status::InternalServerError)))
        }
    }
}

pub fn edit_handler(req: &mut Request) -> IronResult<Response> {
    let login_id = handlers::account::get_login_id(req);
    if login_id == 0 {
        return Ok(Response::with((status::Found, Redirect(url_for!(req, "account/get_signin")))));
    }
    let conn = get_pg_connection!(req);
    let mut resp = Response::new();
    #[derive(Serialize, Default)]
    struct Data {
        logged_in: bool,
        post: models::post::Post,
        tags: String,
    }

    let post: models::post::Post;

    let ref id_str = req.extensions.get::<Router>().unwrap().find("id").unwrap_or("/");
    let id = id_str.parse::<i32>().unwrap();

    match models::post::get_by_id(conn, id) {
        Ok(post_obj) => {
            if post_obj.user_id != login_id {
                return Ok(Response::with((status::Forbidden)));
            }
            post = post_obj;
        },
        Err(e) => {
            println!("Errored: {:?}", e);
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
        post: post,
        tags: tag_str,
    };
    resp.set_mut(Template::new("post/edit", to_json(&data))).set_mut(status::Ok);
    return Ok(resp);
}

pub fn update_handler(req: &mut Request) -> IronResult<Response> {
    let login_id = handlers::account::get_login_id(req);
    if login_id == 0 {
        return Ok(Response::with((status::Found, Redirect(url_for!(req, "account/get_signin")))));
    }

    let conn_s = get_pg_connection!(req);
    let conn_u = get_pg_connection!(req);

    use params::{Params, Value};
    let map = req.get_ref::<Params>().unwrap();

    let id: i32;
    let title: String;
    let body: String;
    let tags: String;

    match map.find(&["id"]) {
        Some(&Value::String(ref name)) => {
            id = name.to_string().parse::<i32>().unwrap();
        },
        _ => return Ok(Response::with((status::BadRequest))),
    }

    match map.find(&["title"]) {
        Some(&Value::String(ref name)) => {
            title = name.to_string();
        },
        _ => return Ok(Response::with((status::BadRequest))),
    }

    match map.find(&["body"]) {
        Some(&Value::String(ref name)) => {
            body = name.to_string();
        },
        _ => return Ok(Response::with((status::BadRequest))),
    }

    match map.find(&["tags"]) {
        Some(&Value::String(ref name)) => {
            tags = name.to_string();
        },
        _ => return Ok(Response::with((status::BadRequest))),
    }

    match models::post::get_by_id(conn_s, id) {
        Ok(post_obj) => {
            if post_obj.user_id != login_id {
                return Ok(Response::with((status::Forbidden)));
            }
        },
        Err(e) => {
            println!("Errored: {:?}", e);
            return Ok(Response::with((status::InternalServerError)));
        }
    }

    let body_db = body.clone();
    let body_slack = body.clone();

    match models::post::update(conn_u, id, title, body_db, tags) {
        Ok(_) => {
            let link = format!("{}/{}/{}", helper::get_domain(), "post/show", id).to_string();
            let text = format!("{}\n{}\n{}", "Edit post", body_slack, link).to_string();
            helper::slack(text);

            let url = Url::parse(&format!("{}/{}/{}", helper::get_domain(), "post/show", id).to_string()).unwrap();
            return Ok(Response::with((status::Found, Redirect(url))));
        },
        Err(e) => {
            println!("Errored: {:?}", e);
            return Ok(Response::with((status::InternalServerError)));
        }
    }
}

pub fn comment_handler(req: &mut Request) -> IronResult<Response> {
    let login_id = handlers::account::get_login_id(req);
    if login_id == 0 {
        return Ok(Response::with((status::Found, Redirect(url_for!(req, "account/get_signin")))));
    }
    let conn = get_pg_connection!(req);

    use params::{Params, Value};
    let map = req.get_ref::<Params>().unwrap();

    let id: i32;
    let body: String;

    match map.find(&["id"]) {
        Some(&Value::String(ref name)) => {
            id = name.parse::<i32>().unwrap();
        },
        _ => return Ok(Response::with((status::BadRequest))),
    }

    match map.find(&["body"]) {
        Some(&Value::String(ref name)) => {
            body = name.to_string();
        },
        _ => return Ok(Response::with((status::BadRequest))),
    }

    let body_db = body.clone();
    let body_slack = body.clone();

    match models::post::add_comment(conn, login_id, id, body_db) {
        Ok(_) => {
            let link = format!("{}/{}/{}", helper::get_domain(), "post/show", id).to_string();
            let text = format!("{}\n{}\n{}", "New comment", body_slack, link).to_string();
            helper::slack(text);

            let url = Url::parse(&format!("{}/{}/{}", helper::get_domain(), "post/show", id).to_string()).unwrap();
            return Ok(Response::with((status::Found, Redirect(url))));
        },
        Err(e) => {
            println!("Errored: {:?}", e);
            return Ok(Response::with((status::InternalServerError)));
        }
    }
}
