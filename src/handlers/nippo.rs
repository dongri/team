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
    resp.set_mut(Template::new("nippo/form", to_json(&data))).set_mut(status::Ok);
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

    {
        use params::{Params, Value};
        let map = req.get_ref::<Params>().unwrap();


        match map.get("title") {
            Some(&Value::String(ref name)) => {
                title = name.to_string();
            },
            _ => title = "".to_string(),
        }
        match map.get("body") {
            Some(&Value::String(ref name)) => {
                body = name.to_string();
            },
            _ => body = "".to_string(),
        }
    }

    match models::nippo::create_nippo(conn, login_id, title, body) {
        Ok(id) => {
            let url = Url::parse(&format!("{}/{}/{}", helper::get_domain(), "/nippo/show", id).to_string()).unwrap();
            return Ok(Response::with((status::Found, Redirect(url))));
            // let path = &format!("{}/{}", "nippo/show", id);
            // return Ok(Response::with((status::Found, Redirect(url_for!(req, path)))));
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
    let mut resp = Response::new();
    #[derive(Serialize)]
    struct Data {
        logged_in: bool,
        nippos: Vec<models::nippo::Nippo>,
    }
    let conn = get_pg_connection!(req);
    match models::nippo::list_nippos(conn) {
        Ok(nippos) => {
            let data = Data {
                logged_in: login_id != 0,
                nippos: nippos,
            };
            resp.set_mut(Template::new("nippo/list", to_json(&data))).set_mut(status::Ok);
            return Ok(resp);
        },
        Err(e) => {
            println!("Errored: {:?}", e);
            Ok(Response::with((status::InternalServerError)))
        }
    }
}

pub fn show_handler(req: &mut Request) -> IronResult<Response> {
    let login_id = handlers::account::get_login_id(req);
    if login_id == 0 {
        return Ok(Response::with((status::Found, Redirect(url_for!(req, "account/get_signin")))));
    }
    let conn = get_pg_connection!(req);
    let conn2 = get_pg_connection!(req);
    let mut resp = Response::new();

    let ref id_str = req.extensions.get::<Router>().unwrap().find("id").unwrap_or("/");
    let id = id_str.parse::<i32>().unwrap();

    #[derive(Serialize, Default)]
    struct Data {
        logged_in: bool,
        nippo: models::nippo::Nippo,
        comments: Vec<models::nippo::Comment>,
    }

    let nippo: models::nippo::Nippo;
    let comments: Vec<models::nippo::Comment>;

    match models::nippo::get_marked_nippo_by_id(conn, id) {
        Ok(nippo_obj) => {
            nippo = nippo_obj;
        },
        Err(e) => {
            println!("Errored: {:?}", e);
            return Ok(Response::with((status::InternalServerError)))
        }
    }

    match models::nippo::get_nippo_comments(conn2, id) {
        Ok(comments_obj) => {
            comments = comments_obj;
        },
        Err(e) => {
            println!("Errored: {:?}", e);
            return Ok(Response::with((status::InternalServerError)))
        }
    }

    let data = Data {
        logged_in: login_id != 0,
        nippo: nippo,
        comments: comments,
    };

    resp.set_mut(Template::new("nippo/show", to_json(&data))).set_mut(status::Ok);
    return Ok(resp);
}

pub fn delete_handler(req: &mut Request) -> IronResult<Response> {
    let login_id = handlers::account::get_login_id(req);
    if login_id == 0 {
        return Ok(Response::with((status::Found, Redirect(url_for!(req, "account/get_signin")))));
    }
    let conn = get_pg_connection!(req);

    let ref id_str = req.extensions.get::<Router>().unwrap().find("id").unwrap_or("/");
    let id = id_str.parse::<i32>().unwrap();

    match models::nippo::delete_nippo_by_id(conn, id) {
        Ok(_) => {
            return Ok(Response::with((status::Found, Redirect(url_for!(req, "nippo/list")))));
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
        nippo: models::nippo::Nippo,
    }

    let nippo: models::nippo::Nippo;

    let ref id_str = req.extensions.get::<Router>().unwrap().find("id").unwrap_or("/");
    let id = id_str.parse::<i32>().unwrap();

    match models::nippo::get_nippo_by_id(conn, id) {
        Ok(nippo_obj) => {
            nippo = nippo_obj;
        },
        Err(e) => {
            println!("Errored: {:?}", e);
            return Ok(Response::with((status::InternalServerError)));
        }
    }
    let data = Data {
        logged_in: login_id != 0,
        nippo: nippo,
    };
    resp.set_mut(Template::new("nippo/edit", to_json(&data))).set_mut(status::Ok);
    return Ok(resp);
}

pub fn update_handler(req: &mut Request) -> IronResult<Response> {
    let login_id = handlers::account::get_login_id(req);
    if login_id == 0 {
        return Ok(Response::with((status::Found, Redirect(url_for!(req, "account/get_signin")))));
    }
    let conn = get_pg_connection!(req);

    use params::{Params, Value};
    let map = req.get_ref::<Params>().unwrap();

    let mut id_str = "";
    let mut title = "";
    let mut body = "";

    match map.find(&["id"]) {
        Some(&Value::String(ref name)) => {
            id_str = name
        },
        _ => print!("{:?}", "a"),
    }
    let id = id_str.parse::<i32>().unwrap();

    match map.find(&["title"]) {
        Some(&Value::String(ref name)) => {
            title = name
        },
        _ => print!("{:?}", "a"),
    }

    match map.find(&["body"]) {
        Some(&Value::String(ref name)) => {
            body = name
        },
        _ => print!("{:?}", "a"),
    }


    match models::nippo::update_nippo(conn, id, title.to_string(), body.to_string()) {
        Ok(_) => {
            let url = Url::parse(&format!("{}/{}/{}", helper::get_domain(), "nippo/show", id).to_string()).unwrap();
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

    let mut id_str = "";
    let mut body = "";

    match map.find(&["id"]) {
        Some(&Value::String(ref name)) => {
            id_str = name
        },
        _ => print!("{:?}", "a"),
    }
    let id = id_str.parse::<i32>().unwrap();

    match map.find(&["body"]) {
        Some(&Value::String(ref name)) => {
            body = name
        },
        _ => print!("{:?}", "a"),
    }

    match models::nippo::add_comment_nippo(conn, login_id, id, body.to_string()) {
        Ok(_) => {
            println!("{:?}", id);
            let url = Url::parse(&format!("{}/{}/{}", helper::get_domain(), "nippo/show", id).to_string()).unwrap();
            return Ok(Response::with((status::Found, Redirect(url))));
        },
        Err(e) => {
            println!("Errored: {:?}", e);
            return Ok(Response::with((status::InternalServerError)));
        }
    }
}
