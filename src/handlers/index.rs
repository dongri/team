use persistent;
use iron::prelude::*;
use iron::status;
use iron::modifiers::Redirect;
use hbs::Template;
use hbs::handlebars::to_json;

use db;
use models;
use handlers;
use helper;

const PAGINATES_PER: i32 = 10;

pub fn index_handler(req: &mut Request) -> IronResult<Response> {
    let conn = get_pg_connection!(req);
    let mut login_user: models::user::User = models::user::User{..Default::default()};
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
        login_user: models::user::User,
        feeds: Vec<models::post::Feed>,
        current_page: i32,
        total_page: i32,
        next_page: i32,
        prev_page: i32,
    }

    let mut page = page_param.parse::<i32>().unwrap();
    let offset = (page - 1) * PAGINATES_PER;
    let limit = PAGINATES_PER;

    let feeds: Vec<models::post::Feed>;
    let count: i32;

    match models::post::get_feeds(&conn, &offset, &limit) {
        Ok(feeds_db) => {
            feeds = feeds_db;
        }
        Err(e) => {
            error!("Errored: {:?}", e);
            return Ok(Response::with((status::InternalServerError)));
        }
    }

    match models::post::get_feed_count(&conn) {
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
        feeds: feeds,
        current_page: page,
        total_page: count / PAGINATES_PER + 1,
        next_page: page + 1,
        prev_page: page - 1,
    };

    resp.set_mut(Template::new("index", to_json(&data)))
        .set_mut(status::Ok);
    return Ok(resp);
}

pub fn search_handler(req: &mut Request) -> IronResult<Response> {
    let conn = get_pg_connection!(req);
    let mut login_user: models::user::User = models::user::User{..Default::default()};
    match handlers::account::current_user(req, &conn) {
        Ok(user) => { login_user = user; }
        Err(e) => { error!("Errored: {:?}", e); }
    }
    let login_id = login_user.id;
    if login_id == 0 {
        return Ok(Response::with((status::Found, Redirect(helper::redirect_url("/signin")))));
    }

    let keyword_param: String;
    let kind_param: String;
    let page_param: String;

    {
        use params::{Params, Value};
        let map = req.get_ref::<Params>().unwrap();
        match map.get("keyword") {
            Some(&Value::String(ref name)) => {
                keyword_param = name.to_string();
            }
            _ => keyword_param = "".to_string(),
        }
        match map.get("kind") {
            Some(&Value::String(ref name)) => {
                kind_param = name.to_string();
            }
            _ => kind_param = "all".to_string(),
        }
        match map.get("page") {
            Some(&Value::String(ref name)) => {
                page_param = name.to_string();
            }
            _ => page_param = "1".to_string(),
        }
    }

    let keyword_search = keyword_param.clone();
    let kind_search = kind_param.clone();
    let keyword_count = keyword_param.clone();
    let kind_count = kind_param.clone();

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
        keyword: String,
    }

    let mut page = page_param.parse::<i32>().unwrap();
    let offset = (page - 1) * PAGINATES_PER;
    let limit = PAGINATES_PER;

    let posts: Vec<models::post::Post>;
    let count: i32;

    match models::post::search(&conn, &keyword_search, &kind_search, &offset, &limit) {
        Ok(posts_db) => {
            posts = posts_db;
        }
        Err(e) => {
            error!("Errored: {:?}", e);
            return Ok(Response::with((status::InternalServerError)));
        }
    }

    match models::post::search_count(&conn, &keyword_count, &kind_count) {
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
        keyword: keyword_param,
    };

    resp.set_mut(Template::new("search", to_json(&data)))
        .set_mut(status::Ok);
    return Ok(resp);
}

pub fn tag_handler(req: &mut Request) -> IronResult<Response> {
    let conn = get_pg_connection!(req);
    let mut login_user: models::user::User = models::user::User{..Default::default()};
    match handlers::account::current_user(req, &conn) {
        Ok(user) => { login_user = user; }
        Err(e) => { error!("Errored: {:?}", e); }
    }
    let login_id = login_user.id;
    if login_id == 0 {
        return Ok(Response::with((status::Found, Redirect(helper::redirect_url("/signin")))));
    }

    let page_param: String;
    let tag_param: String;

    {
        use params::Params;
        let map = &req.get_ref::<Params>().unwrap();

        match helper::get_param(map, "page") {
            Ok(value) => page_param = value,
            Err(_) => page_param = String::from("1"),
        }

        match helper::get_param(map, "name") {
            Ok(value) => tag_param = value,
            Err(st) => return Ok(Response::with((st))),
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
        tag_name: String,
    }

    let mut page = page_param.parse::<i32>().unwrap();
    let offset = ( page - 1 ) * PAGINATES_PER;
    let limit = PAGINATES_PER;

    let posts: Vec<models::post::Post>;
    let count: i32;

    match models::tag::tag_search(&conn, &tag_param, offset, limit) {
        Ok(posts_db) => {
            posts = posts_db;
        },
        Err(e) => {
            error!("Errored: {:?}", e);
            return Ok(Response::with((status::InternalServerError)));
        }
    }

    match models::tag::tag_count(&conn, &tag_param) {
        Ok(count_db) => {
            count = count_db;
        },
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
        tag_name: tag_param,
    };

    resp.set_mut(Template::new("tag", to_json(&data))).set_mut(status::Ok);
    return Ok(resp);
}
