use iron::{Request, status};
use iron::modifiers::{Redirect};
use iron::prelude::{IronResult};
use iron::prelude::*;
use hbs::{Template};
use persistent;
use hbs::handlebars::to_json;

use iron_sessionstorage;
use iron_sessionstorage::traits::*;

use db;
use models;
use helper;
use handlers;

#[derive(Serialize, Debug, Default)]
pub struct Login {
    id: String,
}

impl iron_sessionstorage::Value for Login {
    fn get_key() -> &'static str { "logged_in_user" }
    fn into_raw(self) -> String { self.id }
    fn from_raw(value: String) -> Option<Self> {
        if value.is_empty() {
            None
        } else {
            Some(Login { id: value })
        }
    }
}

pub fn get_signup_handler(_: &mut Request) -> IronResult<Response> {
    let mut resp = Response::new();
    resp.set_mut(Template::new("account/signup", {})).set_mut(status::Ok);
    return Ok(resp);
}

pub fn post_signup_handler(req: &mut Request) -> IronResult<Response> {
    let conn = get_pg_connection!(req);

    let email: String;
    let username: String;
    let mut password: String;

    {
        use params::{Params, Value};
        let map = req.get_ref::<Params>().unwrap();

        match map.get("email") {
            Some(&Value::String(ref name)) => {
                if name == "" {
                    return Ok(Response::with((status::BadRequest)));
                }
                email= name.to_string();
            },
            _ => return Ok(Response::with((status::BadRequest))),
        }
        match map.get("username") {
            Some(&Value::String(ref name)) => {
                if name == "" {
                    return Ok(Response::with((status::BadRequest)));
                }
                username = name.to_string();
            },
            _ => return Ok(Response::with((status::BadRequest))),
        }
        match map.get("password") {
            Some(&Value::String(ref name)) => {
                if name == "" {
                    return Ok(Response::with((status::BadRequest)));
                }
                password = name.to_string();
            },
            _ => return Ok(Response::with((status::BadRequest))),
        }
    }

    password = helper::encrypt_password(password);
    match models::user::create(conn, email, username, password) {
        Ok(_) => {
            return Ok(Response::with((status::Found, Redirect(url_for!(req, "account/get_signin")))));
        },
        Err(e) => {
            println!("Errored: {:?}", e);
            return Ok(Response::with((status::InternalServerError)))
        }
    }
}

pub fn get_signin_handler(req: &mut Request) -> IronResult<Response> {
    let mut resp = Response::new();
    if try!(req.session().get::<Login>()).is_some() {
        // Already logged in
        return Ok(Response::with((status::Found, Redirect(url_for!(req, "nippo/list")))));
    }
    resp.set_mut(Template::new("account/signin", {})).set_mut(status::Ok);
    return Ok(resp);
}

pub fn post_signin_handler(req: &mut Request) -> IronResult<Response> {
    let conn = get_pg_connection!(req);

    let email: String;
    let mut password: String;

    {
        use params::{Params, Value};
        let map = req.get_ref::<Params>().unwrap();

        match map.get("email") {
            Some(&Value::String(ref name)) => {
                email = name.to_string();
            },
            _ => return Ok(Response::with((status::BadRequest))),
        }
        match map.get("password") {
            Some(&Value::String(ref name)) => {
                password = name.to_string();
            },
            _ => return Ok(Response::with((status::BadRequest))),
        }
    }

    password = helper::encrypt_password(password);
    match models::user::get_by_username_password(conn, email, password) {
        Ok(user) => {
            println!("{:?}", user.username);
            if user.username != "" {
                try!(req.session().set(Login { id: user.id.to_string() }));
                return Ok(Response::with((status::Found, Redirect(url_for!(req, "nippo/list")))))
            } else {
                return Ok(Response::with((status::Found, Redirect(url_for!(req, "account/get_signin")))))
            }
        },
        Err(e) => {
            println!("Errored: {:?}", e);
            return Ok(Response::with((status::InternalServerError)))
        }
    }
}

pub fn get_signout_handler(req: &mut Request) -> IronResult<Response> {
    try!(req.session().clear());
    return Ok(Response::with((status::Found, Redirect(url_for!(req, "account/get_signin")))))
}

pub fn get_login_id(req: &mut Request) -> i32 {
    let login = req.session().get::<Login>().ok().and_then(|x| x).unwrap_or(Login{id: "".to_string()});
    if login.id == "" {
        return 0;
    } else {
        return login.id.parse::<i32>().unwrap();
    }
}

pub fn get_settings_handler(req: &mut Request) -> IronResult<Response> {
    let login_id = handlers::account::get_login_id(req);
    if login_id == 0 {
        return Ok(Response::with((status::Found, Redirect(url_for!(req, "account/get_signin")))));
    }
    let conn_s = get_pg_connection!(req);
    let mut resp = Response::new();

    #[derive(Serialize, Default)]
    struct Data {
        logged_in: bool,
        user: models::user::User,
    }

    let user: models::user::User;

    match models::user::get_by_id(conn_s, login_id) {
        Ok(user_obj) => {
            user = user_obj;
        },
        Err(e) => {
            println!("Errored: {:?}", e);
            return Ok(Response::with((status::InternalServerError)))
        }
    }
    let data = Data {
        logged_in: login_id != 0,
        user: user,
    };

    resp.set_mut(Template::new("account/settings", to_json(&data))).set_mut(status::Ok);
    return Ok(resp);
}

pub fn post_settings_handler(req: &mut Request) -> IronResult<Response> {
    let login_id = handlers::account::get_login_id(req);
    if login_id == 0 {
        return Ok(Response::with((status::Found, Redirect(url_for!(req, "account/get_signin")))));
    }
    let conn = get_pg_connection!(req);
    let icon_url: String;
    {
        use params::{Params, Value};
        let map = req.get_ref::<Params>().unwrap();

        match map.get("icon_url") {
            Some(&Value::String(ref name)) => {
                icon_url = name.to_string();
            },
            _ => return Ok(Response::with((status::BadRequest))),
        }
    }

    match models::user::update_icon_url(conn, login_id, icon_url) {
        Ok(_) => {
            return Ok(Response::with((status::Found, Redirect(url_for!(req, "account/get_settings")))));
        },
        Err(e) => {
            println!("Errored: {:?}", e);
            return Ok(Response::with((status::InternalServerError)));
        }
    }
}
