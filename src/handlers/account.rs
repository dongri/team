use iron::{Request, status};
use iron::modifiers::Redirect;
use iron::prelude::IronResult;
use iron::prelude::*;
use router::Router;
use hbs::Template;
use persistent;
use hbs::handlebars::to_json;

use iron_sessionstorage;
use iron_sessionstorage::traits::*;

use db;
use models;
use helper;
use handlers;
use env;

const PAGINATES_PER: i32 = 10;

#[derive(Serialize, Debug, Default)]
pub struct Login {
    id: String,
}

impl iron_sessionstorage::Value for Login {
    fn get_key() -> &'static str {
        "logged_in_user"
    }
    fn into_raw(self) -> String {
        self.id
    }
    fn from_raw(value: String) -> Option<Self> {
        if value.is_empty() {
            None
        } else {
            Some(Login { id: value })
        }
    }
}

#[derive(Serialize, Debug, Default)]
pub struct RefUrl {
    url: String,
}

impl iron_sessionstorage::Value for RefUrl {
    fn get_key() -> &'static str {
        "ref_url"
    }
    fn into_raw(self) -> String {
        self.url
    }
    fn from_raw(value: String) -> Option<Self> {
        if value.is_empty() {
            None
        } else {
            Some(RefUrl { url: value })
        }
    }
}

pub fn get_signup_handler(_: &mut Request) -> IronResult<Response> {
    let mut resp = Response::new();
    resp.set_mut(Template::new("account/signup", {}))
        .set_mut(status::Ok);
    return Ok(resp);
}

pub fn post_signup_handler(req: &mut Request) -> IronResult<Response> {
    let conn = get_pg_connection!(req);

    let username: String;
    let mut password: String;

    {
        use params::{Params, Value};
        let map = req.get_ref::<Params>().unwrap();

        match map.get("username") {
            Some(&Value::String(ref name)) => {
                if name == "" {
                    return Ok(Response::with(status::BadRequest));
                }
                username = name.to_string();
            }
            _ => return Ok(Response::with(status::BadRequest)),
        }
        match map.get("password") {
            Some(&Value::String(ref name)) => {
                if name == "" {
                    return Ok(Response::with(status::BadRequest));
                }
                password = name.to_string();
            }
            _ => return Ok(Response::with(status::BadRequest)),
        }
    }

    password = helper::encrypt_password(password);
    match models::user::create(&conn, &username, &password) {
        Ok(user_id) => {
            try!(req.session().set(Login { id: user_id.to_string() }));
            return Ok(Response::with((status::Found, Redirect(helper::redirect_url("/")))));
        }
        Err(e) => {
            info!("Errored: {:?}", e);
            return Ok(Response::with(status::InternalServerError));
        }
    }
}

pub fn get_signin_handler(req: &mut Request) -> IronResult<Response> {
    let mut resp = Response::new();
    if try!(req.session().get::<Login>()).is_some() {
        // Already logged in
        return Ok(Response::with((status::Found, Redirect(helper::redirect_url("/")))));
    }
    resp.set_mut(Template::new("account/signin", {}))
        .set_mut(status::Ok);
    return Ok(resp);
}

pub fn post_signin_handler(req: &mut Request) -> IronResult<Response> {
    let conn = get_pg_connection!(req);

    let username: String;
    let mut password: String;

    {
        use params::{Params, Value};
        let map = req.get_ref::<Params>().unwrap();

        match map.get("username") {
            Some(&Value::String(ref name)) => {
                username = name.to_string();
            }
            _ => return Ok(Response::with(status::BadRequest)),
        }
        match map.get("password") {
            Some(&Value::String(ref name)) => {
                password = name.to_string();
            }
            _ => return Ok(Response::with(status::BadRequest)),
        }
    }

    password = helper::encrypt_password(password);
    match models::user::get_by_username_password(&conn, &username, &password) {
        Ok(user) => {
            if user.username != "" {
                try!(req.session().set(Login { id: user.id.to_string() }));
                let ref_url = req.session().get::<RefUrl>().ok().and_then(|x| x)
                    .unwrap_or(RefUrl { url: "".to_string() });
                let url = Url::parse(&format!("{}", ref_url.url)
                    .to_string())
                    .unwrap_or(helper::redirect_url("/"));
                println!("==== Referer: {:?} =====", url.to_string());
                return Ok(Response::with((status::Found, Redirect(url))));
            } else {
                return Ok(Response::with((status::Found,
                                          Redirect(helper::redirect_url("/signin")))));
            }
        }
        Err(e) => {
            error!("Errored: {:?}", e);
            return Ok(Response::with(status::InternalServerError));
        }
    }
}

pub fn get_signout_handler(req: &mut Request) -> IronResult<Response> {
    try!(req.session().clear());
    return Ok(Response::with((status::Found, Redirect(helper::redirect_url("/signin")))));
}

pub fn get_settings_handler(req: &mut Request) -> IronResult<Response> {
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

    #[derive(Serialize, Debug, Default)]
    struct Menu {
        name: String,
        checked: String,
    }

    #[derive(Serialize, Default)]
    struct Data {
        logged_in: bool,
        user: models::user::User,
        login_user: models::user::UserWithPreference,
        menu: Vec<Menu>,
        theme_light: String,
        theme_black: String,
    }

    let user: models::user::User;

    match models::user::get_by_id(&conn, &login_id) {
        Ok(user_obj) => {
            user = user_obj;
        }
        Err(e) => {
            error!("Errored: {:?}", e);
            return Ok(Response::with(status::InternalServerError));
        }
    }

    // Menu
    let default_menu = &env::CONFIG.team_menu;
    let mut menu: Vec<Menu> = Vec::new();
    for name in default_menu.split(",") {
        let mut m = Menu{
            name: "".to_string(),
            checked: "".to_string(),
        };
        m.name = name.to_string();
        if login_user.menu.contains(&m.name){
            m.checked = "checked".to_string();
        }
        menu.push(m);
    }

    // Theme
    let mut theme_light_checked = String::from("");
    let mut theme_black_checked = String::from("");
    if login_user.theme == "light" {
        theme_light_checked = String::from("checked");
    }
    if login_user.theme == "black" {
        theme_black_checked = String::from("checked");
    }
    
    let data = Data {
        logged_in: login_id != 0,
        login_user: login_user,
        user: user,
        menu: menu,
        theme_light: theme_light_checked,
        theme_black: theme_black_checked,
    };

    resp.set_mut(Template::new("account/settings", to_json(&data)))
        .set_mut(status::Ok);
    return Ok(resp);
}

pub fn post_settings_handler(req: &mut Request) -> IronResult<Response> {
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

    let icon_url: String;
    {
        use params::{Params, Value};
        let map = req.get_ref::<Params>().unwrap();

        match map.get("icon_url") {
            Some(&Value::String(ref name)) => {
                icon_url = name.to_string();
            }
            _ => return Ok(Response::with(status::BadRequest)),
        }
    }

    match models::user::update_icon_url(&conn, &login_id, &icon_url) {
        Ok(_) => {
            return Ok(Response::with((status::Found,
                                      Redirect(helper::redirect_url("/account/settings")))));
        }
        Err(e) => {
            error!("Errored: {:?}", e);
            return Ok(Response::with(status::InternalServerError));
        }
    }
}

pub fn post_password_update(req: &mut Request) -> IronResult<Response> {
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

    let current_password: String;
    let new_password: String;
    let confirm_password: String;
    {
        use params::Params;

        let map = &req.get_ref::<Params>().unwrap();
        match helper::get_param(map, "current_password") {
            Ok(value) => current_password = value,
            Err(st) => return Ok(Response::with(st)),
        }

        match helper::get_param(map, "new_password") {
            Ok(value) => new_password = value,
            Err(st) => return Ok(Response::with(st)),
        }

        match helper::get_param(map, "confirm_password") {
            Ok(value) => confirm_password = value,
            Err(st) => return Ok(Response::with(st)),
        }
    }

    if new_password != confirm_password {
        return Ok(Response::with(status::BadRequest));
    }

    let current_password = helper::encrypt_password(current_password);
    let user: models::user::UserWithPassword;

    match models::user::get_with_password_by_id(&conn, &login_id) {
        Ok(u) => user = u,
        Err(_) => return Ok(Response::with(status::BadRequest)),
    }

    if current_password != user.password {
        return Ok(Response::with(status::BadRequest));
    }

    match models::user::update_password(&conn, &login_id, &helper::encrypt_password(new_password)) {
        Ok(_) => {
            return Ok(Response::with((status::Found,
                                      Redirect(helper::redirect_url("/account/settings")))));
        }
        Err(e) => {
            error!("Errored: {:?}", e);
            return Ok(Response::with(status::InternalServerError));
        }
    }
}

pub fn post_username_update(req: &mut Request) -> IronResult<Response> {
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

    let username: String;
    {
        use params::Params;
        let map = &req.get_ref::<Params>().unwrap();
        match helper::get_param(map, "username") {
            Ok(value) => username = value,
            Err(st) => return Ok(Response::with(st)),
        }
    }

    match models::user::update_username(&conn, &login_id, &username) {
        Ok(_) => {
            return Ok(Response::with((status::Found,
                                      Redirect(helper::redirect_url("/account/settings")))));
        }
        Err(e) => {
            error!("Errored: {:?}", e);
            return Ok(Response::with(status::InternalServerError));
        }
    }
}

pub fn current_user(req: &mut Request, conn: &db::PostgresConnection) -> Result<models::user::UserWithPreference, String> {
    let mut url = req.url.to_string();
    if env::CONFIG.team_domain.starts_with("https") {
        url = url.replace("http", "https")
    }
    let _ = req.session().set(RefUrl { url: url } );
    let mut user: models::user::UserWithPreference = models::user::UserWithPreference{..Default::default()};
    let login = req.session()
        .get::<Login>()
        .ok()
        .and_then(|x| x)
        .unwrap_or(Login { id: "".to_string() });

    if login.id == "" {
        return Ok(user);
    } else {
        let login_id = login.id.parse::<i32>().unwrap();
        match models::user::get_current_user(&conn, &login_id) {
            Ok(user_obj) => {
                user = user_obj;
                Ok(user)
            }
            Err(e) => {
                error!("Errored: {:?}", e);
                Err(format!("{}", e))
            }
        }
    }
}

pub fn profile_post_handler(req: &mut Request) -> IronResult<Response> {
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

    let ref username = req.extensions
        .get::<Router>()
        .unwrap()
        .find("username")
        .unwrap_or("/");

    #[derive(Serialize, Debug)]
    struct Data {
        logged_in: bool,
        login_user: models::user::UserWithPreference,
        user: models::user::User,
        posts: Vec<models::post::Post>,
        current_page: i32,
        total_page: i32,
        next_page: i32,
        prev_page: i32,
        kind: String,
        kind_post_active: String,
        kind_nippo_active: String,
    }

    let mut page = page_param.parse::<i32>().unwrap();
    let offset = (page - 1) * PAGINATES_PER;
    let limit = PAGINATES_PER;

    let user: models::user::User;
    let posts: Vec<models::post::Post>;
    let count: i32;

    match models::user::get_by_username(&conn, &username) {
        Ok(user_db) => {
            user = user_db;
        }
        Err(e) => {
            error!("Errored: {:?}", e);
            return Ok(Response::with(status::InternalServerError));
        }
    }

    match models::post::user_posts(&conn, &username, &offset, &limit, "post") {
        Ok(posts_db) => {
            posts = posts_db;
        }
        Err(e) => {
            error!("Errored: {:?}", e);
            return Ok(Response::with(status::InternalServerError));
        }
    }

    match models::post::user_posts_count(&conn, username, "post") {
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
        user: user,
        posts: posts,
        current_page: page,
        total_page: count / PAGINATES_PER + 1,
        next_page: page + 1,
        prev_page: page - 1,
        kind: String::from("post"),
        kind_post_active: String::from("is-active"),
        kind_nippo_active: String::from(""),
    };

    resp.set_mut(Template::new("account/profile", to_json(&data)))
        .set_mut(status::Ok);
    return Ok(resp);
}

pub fn profile_nippo_handler(req: &mut Request) -> IronResult<Response> {
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

    let ref username = req.extensions
        .get::<Router>()
        .unwrap()
        .find("username")
        .unwrap_or("/");

    #[derive(Serialize, Debug)]
    struct Data {
        logged_in: bool,
        login_user: models::user::UserWithPreference,
        user: models::user::User,
        posts: Vec<models::post::Post>,
        current_page: i32,
        total_page: i32,
        next_page: i32,
        prev_page: i32,
        kind: String,
        kind_post_active: String,
        kind_nippo_active: String,
    }

    let mut page = page_param.parse::<i32>().unwrap();
    let offset = (page - 1) * PAGINATES_PER;
    let limit = PAGINATES_PER;

    let user: models::user::User;
    let posts: Vec<models::post::Post>;
    let count: i32;

    match models::user::get_by_username(&conn, &username) {
        Ok(user_db) => {
            user = user_db;
        }
        Err(e) => {
            error!("Errored: {:?}", e);
            return Ok(Response::with(status::InternalServerError));
        }
    }

    match models::post::user_posts(&conn, &username, &offset, &limit, "nippo") {
        Ok(posts_db) => {
            posts = posts_db;
        }
        Err(e) => {
            error!("Errored: {:?}", e);
            return Ok(Response::with(status::InternalServerError));
        }
    }

    match models::post::user_posts_count(&conn, username, "nippo") {
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
        user: user,
        posts: posts,
        current_page: page,
        total_page: count / PAGINATES_PER + 1,
        next_page: page + 1,
        prev_page: page - 1,
        kind: String::from("nippo"),
        kind_post_active: String::from(""),
        kind_nippo_active: String::from("is-active"),
    };

    resp.set_mut(Template::new("account/profile", to_json(&data)))
        .set_mut(status::Ok);
    return Ok(resp);
}

use oauth2::Config;
use iron::Url;

pub fn get_auth_google_handler(req: &mut Request) -> IronResult<Response> {

    let google_client_id = env::CONFIG.team_google_client_id.as_str();
    let google_client_secret = env::CONFIG.team_google_client_secret.as_str();
    let auth_url = "https://accounts.google.com/o/oauth2/v2/auth";
    let token_url = "https://www.googleapis.com/oauth2/v3/token";
    let mut config = Config::new(google_client_id, google_client_secret, auth_url, token_url);
    // config = config.add_scope("https://www.googleapis.com/auth/plus.me");
    config = config.add_scope("https://www.googleapis.com/auth/userinfo.email");
    config = config.set_redirect_url(env::CONFIG.team_google_redirect_url.as_str());
    config = config.set_state("S5nHXBzfeJBWmE9CmzLKLaFxfjwxqdvAyHPnFnS9");
    let authorize_url = config.authorize_url();

    let code: String;
    {
        use params::{Params, Value};
        let map = req.get_ref::<Params>().unwrap();
        match map.get("code") {
            Some(&Value::String(ref name)) => {
                code = name.to_string();
            }
            _ => code = "".to_string(),
        }
    }

    if code == "" {
        let url = Url::parse(&format!("{}",authorize_url).to_string()).unwrap();
        return Ok(Response::with((status::Found, Redirect(url))));
    } else {
        let result = config.exchange_code(code);
        match result {
            Ok(token) => {
                let allow_domain = env::CONFIG.team_google_allow_domain.as_str();
                let email = helper::get_google_email(token.access_token.to_string());
                let v: Vec<&str> = email.as_str().split("@").collect();
                let username = v[0].to_string();
                let domain = v[1];
                if allow_domain != domain && allow_domain != "" {
                    return Ok(Response::with((status::InternalServerError, "domain error")));
                }

                let ref_url = req.session().get::<RefUrl>().ok().and_then(|x| x)
                    .unwrap_or(RefUrl { url: "".to_string() });
                let url = Url::parse(&format!("{}", ref_url.url)
                    .to_string())
                    .unwrap_or(helper::redirect_url("/"));

                let conn = get_pg_connection!(req);
                let user: models::user::UserWithEmail;
                match models::user::get_by_email(&conn, &email) {
                    Ok(user_db) => {
                        user = user_db;
                        println!("{:?}", user);
                    }
                    Err(e) => {
                        error!("Errored: {:?}", e);
                        return Ok(Response::with(status::InternalServerError));
                    }
                }
                if user.username == "" {
                    match models::user::create_with_email(&conn, &username, &email) {
                        Ok(user_id) => {
                            try!(req.session().set(Login { id: user_id.to_string() }));
                            return Ok(Response::with((status::Found, Redirect(url))));
                        }
                        Err(e) => {
                            info!("Errored: {:?}", e);
                            return Ok(Response::with(status::InternalServerError));
                        }
                    }
                } else {
                    try!(req.session().set(Login { id: user.id.to_string() }));
                    return Ok(Response::with((status::Found, Redirect(url))));
                }
            }
            Err(err) => {
                error!("error: {}", err);
            }
        }
    };

    return Ok(Response::with((status::Found, Redirect(helper::redirect_url("/signin")))));
}

pub fn post_preference_menu(req: &mut Request) -> IronResult<Response> {
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

    let menu: String;
    {
        use params::Params;
        let map = &req.get_ref::<Params>().unwrap();
        match helper::get_param(map, "menu_param") {
            Ok(value) => menu = value,
            Err(st) => return Ok(Response::with(st)),
        }
    }

    match models::user::set_preference_menu(&conn, &login_id, &menu) {
        Ok(_) => {
            return Ok(Response::with((status::Found,
                                      Redirect(helper::redirect_url("/account/settings")))));
        }
        Err(e) => {
            error!("Errored: {:?}", e);
            return Ok(Response::with(status::InternalServerError));
        }
    }
}

pub fn post_preference_theme(req: &mut Request) -> IronResult<Response> {
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

    let theme: String;
    {
        use params::Params;
        let map = &req.get_ref::<Params>().unwrap();
        match helper::get_param(map, "theme") {
            Ok(value) => theme = value,
            Err(st) => return Ok(Response::with(st)),
        }
    }

    match models::user::set_preference_theme(&conn, &login_id, &theme) {
        Ok(_) => {
            return Ok(Response::with((status::Found,
                                      Redirect(helper::redirect_url("/account/settings")))));
        }
        Err(e) => {
            error!("Errored: {:?}", e);
            return Ok(Response::with(status::InternalServerError));
        }
    }
}

use time;
use std::fs;
use iron::mime::Mime;

use env::CONFIG;

pub fn icon_upload_handler(req: &mut Request) -> IronResult<Response> {
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
    let filepath = format!("public/img/profile/icon/{:?}.png", timestamp);
    let fileurl = format!("img/profile/icon/{:?}.png", timestamp);

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