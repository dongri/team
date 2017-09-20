use iron::prelude::*;
use iron::status;
use iron::Url;
use iron::modifiers::Redirect;
use router::Router;
use db;
use persistent;

use env::CONFIG;
use handlers;
use helper;
use models;

pub fn comment_handler(req: &mut Request) -> IronResult<Response> {
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

    let id: i32;
    let body: String;

    use params::{Params, Value};
    {
        let map = req.get_ref::<Params>().unwrap();

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
    }

    let ref kind = req.extensions
        .get::<Router>()
        .unwrap()
        .find("kind")
        .unwrap_or("/");

    let mut mentions = Vec::new();

    match models::post::get_by_id(&conn, &id) {
        Ok(post_obj) => {
            mentions.push(post_obj.user.username);
        }
        Err(e) => {
            error!("Errored: {:?}", e);
            return Ok(Response::with((status::InternalServerError)));
        }
    }

    match models::post::get_comments_by_post_id(&conn, &id) {
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
            return Ok(Response::with((status::InternalServerError)));
        }
    }


    match models::post::add_comment(&conn, &login_id, &id, &body) {
        Ok(_) => {
            let title = String::from("New comment");
            helper::post_to_slack(&conn, &login_id, &title, &body, &id, mentions);
            let url = Url::parse(&format!("{}/{}/show/{}", &CONFIG.team_domain, kind, id)
                                     .to_string()).unwrap();
            return Ok(Response::with((status::Found, Redirect(url))));
        }
        Err(e) => {
            error!("Errored: {:?}", e);
            return Ok(Response::with((status::InternalServerError)));
        }
    }
}

pub fn comment_update_handler(req: &mut Request) -> IronResult<Response> {
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

    let id: i32;
    let action: String;
    let body: String;
    let comment: models::post::Comment;

    {
        let ref id_str = req.extensions
            .get::<Router>()
            .unwrap()
            .find("id")
            .unwrap_or("/");
        id = id_str.parse::<i32>().unwrap();
    }

    use params::{Params, Value};
    {
        let map = req.get_ref::<Params>().unwrap();

        match map.find(&["action"]) {
            Some(&Value::String(ref name)) => {
                action = name.to_string();
            }
            _ => return Ok(Response::with((status::BadRequest))),
        }

        match map.find(&["body"]) {
            Some(&Value::String(ref name)) => {
                body = name.to_string();
            }
            _ => return Ok(Response::with((status::BadRequest))),
        }
    }

    let ref kind = req.extensions
        .get::<Router>()
        .unwrap()
        .find("kind")
        .unwrap_or("/");

    match models::post::get_comment_by_id(&conn, &id) {
        Ok(db_comment) => {
            comment = db_comment;
            if comment.user_id != login_id {
                return Ok(Response::with((status::Forbidden)));
            }
        }
        Err(e) => {
            error!("Errored: {:?}", e);
            return Ok(Response::with((status::InternalServerError)));
        }
    }

    if action == "update" {
        match models::post::update_comment_by_id(&conn, &id, &body) {
            Ok(_) => {
                let url = Url::parse(&format!("{}/{}/show/{}", &CONFIG.team_domain, kind, comment.post_id)
                                         .to_string()).unwrap();
                return Ok(Response::with((status::Found, Redirect(url))));
            }
            Err(e) => {
                error!("Errored: {:?}", e);
                return Ok(Response::with((status::InternalServerError)));
            }
        }
    }
    if action == "delete" {
        match models::post::delete_comment_by_id(&conn, &id) {
            Ok(_) => {
                let url = Url::parse(&format!("{}/{}/show/{}", &CONFIG.team_domain, kind, comment.post_id)
                                         .to_string()).unwrap();
                return Ok(Response::with((status::Found, Redirect(url))));
            }
            Err(e) => {
                error!("Errored: {:?}", e);
                return Ok(Response::with((status::InternalServerError)));
            }
        }
    }
    return Ok(Response::with((status::InternalServerError)));
}
