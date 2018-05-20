use iron::{Request, status};
use iron::modifiers::Redirect;
use iron::prelude::IronResult;
use iron::prelude::*;
use iron::Url;
use router::Router;
use hbs::Template;
use persistent;
use hbs::handlebars::to_json;

use db;
use models;
use helper;
use handlers;
use env::CONFIG;

pub const PAGINATES_PER: i32 = 10;


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

    let mut resp = Response::new();

    #[derive(Serialize, Debug)]
    struct Data {
        logged_in: bool,
        login_user: models::user::UserWithPreference,
        tweets: Vec<models::tweet::Tweet>,
        current_page: i32,
        total_page: i32,
        next_page: i32,
        prev_page: i32,
    }

    let mut page = page_param.parse::<i32>().unwrap();
    if page <= 0 {
        page = 1;
    }
    let offset = (page - 1) * PAGINATES_PER;
    let limit = PAGINATES_PER;

    let tweets: Vec<models::tweet::Tweet>;
    let count: i32;

    match models::tweet::list(&conn, &offset, &limit) {
        Ok(tweets_db) => {
            tweets = tweets_db;
        }
        Err(e) => {
            error!("Errored: {:?}", e);
            return Ok(Response::with(status::InternalServerError));
        }
    }

    match models::tweet::count(&conn) {
        Ok(count_db) => {
            count = count_db;
        }
        Err(e) => {
            error!("Errored: {:?}", e);
            return Ok(Response::with(status::InternalServerError));
        }
    }

    let data = Data {
        logged_in: login_id != 0,
        login_user: login_user,
        tweets: tweets,
        current_page: page,
        total_page: count / PAGINATES_PER + 1,
        next_page: page + 1,
        prev_page: page - 1,
    };

    resp.set_mut(Template::new("tweet/list", to_json(&data)))
        .set_mut(status::Ok);
    return Ok(resp);
}

pub fn post_handler(req: &mut Request) -> IronResult<Response> {
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

    let body: String;

    {
        use params::{Params, Value};
        let map = req.get_ref::<Params>().unwrap();

        match map.get("body") {
            Some(&Value::String(ref name)) => {
                body = name.to_string();
            }
            _ => return Ok(Response::with(status::BadRequest)),
        }
    }

    match models::tweet::create(&conn, &login_id, &body) {
        Ok(id) => {
            let title = String::from("New Tweet");
            let path = String::from("tweet");
            let body = &format!("{}{}{}", "```\n", &body, "\n```").to_string();
            helper::post_to_slack(&conn, &login_id, &title, &body, &id, Vec::new(), &path);
            let url = Url::parse(&format!("{}/tweet/list", &CONFIG.team_domain)
                                     .to_string()).unwrap();
            return Ok(Response::with((status::Found, Redirect(url))));
        }
        Err(e) => {
            error!("Errored: {:?}", e);
            return Ok(Response::with(status::InternalServerError));
        }
    }
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

    #[derive(Serialize)]
    struct Data {
        logged_in: bool,
        login_user: models::user::UserWithPreference,
        tweet: models::tweet::Tweet,
        comments: Vec<models::tweet::Comment>,
    }

    let tweet: models::tweet::Tweet;
    let comments: Vec<models::tweet::Comment>;

    match models::tweet::get_by_id(&conn, &id) {
        Ok(tweet_obj) => {
            tweet = tweet_obj;
        }
        Err(e) => {
            error!("Errored: {:?}", e);
            return Ok(Response::with(status::InternalServerError));
        }
    }

    match models::tweet::get_comments_by_tweet_id(&conn, &id) {
        Ok(comments_obj) => {
            comments = comments_obj;
        }
        Err(e) => {
            error!("Errored: {:?}", e);
            return Ok(Response::with(status::InternalServerError));
        }
    }

    let data = Data {
        logged_in: login_id != 0,
        login_user: login_user,
        tweet: tweet,
        comments: comments,
    };

    resp.set_mut(Template::new("tweet/show", to_json(&data)))
        .set_mut(status::Ok);
    return Ok(resp);
}

pub fn comment_handler(req: &mut Request) -> IronResult<Response> {
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

    let id: i32;
    let body: String;

    use params::{Params, Value};
    {
        let map = req.get_ref::<Params>().unwrap();

        match map.find(&["id"]) {
            Some(&Value::String(ref name)) => {
                id = name.parse::<i32>().unwrap();
            }
            _ => return Ok(Response::with(status::BadRequest)),
        }

        match map.find(&["body"]) {
            Some(&Value::String(ref name)) => {
                body = name.to_string();
            }
            _ => return Ok(Response::with(status::BadRequest)),
        }
    }

    let mut mentions = Vec::new();

    match models::tweet::get_by_id(&conn, &id) {
        Ok(tweet_obj) => {
            mentions.push(tweet_obj.user.username);
        }
        Err(e) => {
            error!("Errored: {:?}", e);
            return Ok(Response::with(status::InternalServerError));
        }
    }

    match models::tweet::get_comments_by_tweet_id(&conn, &id) {
        Ok(comments_obj) => {
            for comment in comments_obj {
                let username: String = comment.user.username;
                if !mentions.contains(&username) {
                    mentions.push(username)
                }
            }
        }
        Err(e) => {
            error!("Errored: {:?}", e);
            return Ok(Response::with(status::InternalServerError));
        }
    }


    match models::tweet::add_comment(&conn, &login_id, &id, &body) {
        Ok(_) => {
            let title = String::from("New comment");
            let path = String::from("tweet");
            helper::post_to_slack(&conn, &login_id, &title, &body, &id, mentions, &path);
            let url = Url::parse(&format!("{}/tweet/show/{}", &CONFIG.team_domain, id)
                                     .to_string()).unwrap();
            return Ok(Response::with((status::Found, Redirect(url))));
        }
        Err(e) => {
            error!("Errored: {:?}", e);
            return Ok(Response::with(status::InternalServerError));
        }
    }
}
