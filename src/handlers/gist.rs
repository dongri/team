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

    let mut resp = Response::new();

    #[derive(Serialize)]
    struct Data {
        logged_in: bool,
        login_user: models::user::UserWithPreference,
    }
    let data = Data {
        logged_in: login_id != 0,
        login_user: login_user,
    };
    resp.set_mut(helper::template("gist/form", to_json(&data)))
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

    let description: String;
    let filename: String;
    let code: String;

    {
        use params::{Params, Value};
        let map = req.get_ref::<Params>().unwrap();

        match map.get("gist-description") {
            Some(&Value::String(ref name)) => {
                description = name.to_string();
            }
            _ => return Ok(Response::with(status::BadRequest)),
        }

        match map.get("gist-filename") {
            Some(&Value::String(ref name)) => {
                filename = name.to_string();
            }
            _ => return Ok(Response::with(status::BadRequest)),
        }
        match map.get("gist-code") {
            Some(&Value::String(ref name)) => {
                code = name.to_string();
            }
            _ => return Ok(Response::with(status::BadRequest)),
        }
    }

    match models::gist::create(&conn, &login_id, &description, &filename, &code) {
        Ok(id) => {
            let title = String::from("New gist");
            let path = String::from("gist");
            let code = &format!("{}{}{}", "```\n", &code, "\n```").to_string();
            helper::post_to_slack(&conn, &login_id, &title, &code, &id, Vec::new(), &path);
            let url = Url::parse(&format!("{}/gist/show/{}", &CONFIG.team_domain, id)
                                     .to_string()).unwrap();
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

    let mut resp = Response::new();

    #[derive(Serialize, Debug)]
    struct Data {
        logged_in: bool,
        login_user: models::user::UserWithPreference,
        gists: Vec<models::gist::Gist>,
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

    let gists: Vec<models::gist::Gist>;
    let count: i32;

    match models::gist::list(&conn, &offset, &limit) {
        Ok(gists_db) => {
            gists = gists_db;
        }
        Err(e) => {
            error!("Errored: {:?}", e);
            return Ok(Response::with(status::InternalServerError));
        }
    }

    match models::gist::count(&conn) {
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
        gists: gists,
        current_page: page,
        total_page: count / PAGINATES_PER + 1,
        next_page: page + 1,
        prev_page: page - 1,
    };

    resp.set_mut(Template::new("gist/list", to_json(&data)))
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

    #[derive(Serialize, Debug, Default)]
    struct GistComment {
        comment: models::gist::Comment,
        editable: bool,
    }

    #[derive(Serialize)]
    struct Data {
        logged_in: bool,
        login_user: models::user::UserWithPreference,
        gist: models::gist::Gist,
        editable: bool,
        deletable: bool,
        comments: Vec<GistComment>,
    }

    let gist: models::gist::Gist;
    let comments: Vec<models::gist::Comment>;

    match models::gist::get_by_id(&conn, &id) {
        Ok(gist_obj) => {
            gist = gist_obj;
        }
        Err(e) => {
            error!("Errored: {:?}", e);
            return Ok(Response::with(status::InternalServerError));
        }
    }

    match models::gist::get_comments_by_gist_id(&conn, &id) {
        Ok(comments_obj) => {
            comments = comments_obj;
        }
        Err(e) => {
            error!("Errored: {:?}", e);
            return Ok(Response::with(status::InternalServerError));
        }
    }

    let mut gist_comments: Vec<GistComment> = Vec::new();
    for comment in comments {
        let owner_id = comment.user_id;
        let pc = GistComment{
            comment: comment,
            editable: owner_id == login_id,
        };
        gist_comments.push(pc);
    }

    let owner_id = gist.user_id;
    let deletable = owner_id == login_id;
    let editable = owner_id == login_id;
    let data = Data {
        logged_in: login_id != 0,
        login_user: login_user,
        gist: gist,
        editable: editable,
        deletable: deletable,
        comments: gist_comments,
    };

    resp.set_mut(Template::new("gist/show", to_json(&data)))
        .set_mut(status::Ok);
    return Ok(resp);
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
        gist: models::gist::Gist,
    }

    let gist: models::gist::Gist;

    let ref id_str = req.extensions
        .get::<Router>()
        .unwrap()
        .find("id")
        .unwrap_or("/");
    let id = id_str.parse::<i32>().unwrap();

    match models::gist::get_by_id(&conn, &id) {
        Ok(gist_obj) => {
            if gist_obj.user_id != login_id {
                return Ok(Response::with(status::Forbidden));
            }
            gist = gist_obj;
        }
        Err(e) => {
            error!("Errored: {:?}", e);
            return Ok(Response::with(status::InternalServerError));
        }
    }

    let data = Data {
        logged_in: login_id != 0,
        login_user: login_user,
        gist: gist,
    };
    resp.set_mut(Template::new("gist/edit", to_json(&data)))
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
    let description: String;
    let filename: String;
    let code: String;

    let old_gist: models::gist::Gist;
    {
        let map = req.get_ref::<Params>().unwrap();
        match map.find(&["id"]) {
            Some(&Value::String(ref name)) => {
                id = name.to_string().parse::<i32>().unwrap();
            }
            _ => return Ok(Response::with(status::BadRequest)),
        }

        match map.find(&["gist-description"]) {
            Some(&Value::String(ref name)) => {
                description = name.to_string();
            }
            _ => return Ok(Response::with(status::BadRequest)),
        }

        match map.find(&["gist-filename"]) {
            Some(&Value::String(ref name)) => {
                filename = name.to_string();
            }
            _ => return Ok(Response::with(status::BadRequest)),
        }

        match map.find(&["gist-code"]) {
            Some(&Value::String(ref name)) => {
                code = name.to_string();
            },
            _ => return Ok(Response::with(status::BadRequest)),
        }
    }

    match models::gist::get_by_id(&conn, &id) {
        Ok(gist_obj) => {
            old_gist = gist_obj;
            if old_gist.user_id != login_id {
                return Ok(Response::with(status::Forbidden));
            }
        }
        Err(e) => {
            error!("Errored: {:?}", e);
            return Ok(Response::with(status::InternalServerError));
        }
    }

    match models::gist::update(&conn, &id, &description, &filename, &code) {
        Ok(_) => {
            let title = String::from("Edit gist");
            let path = String::from("post");
            let code = &format!("{}{}{}", "```\n", &code, "\n```").to_string();
            helper::post_to_slack(&conn, &login_id, &title, &code, &id, Vec::new(), &path);
            let url = Url::parse(&format!("{}/gist/show/{}", &CONFIG.team_domain, id)
                                     .to_string()).unwrap();
            return Ok(Response::with((status::Found, Redirect(url))));
        }
        Err(e) => {
            error!("Errored: {:?}", e);
            return Ok(Response::with(status::InternalServerError));
        }
    }
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

    match models::gist::get_by_id(&conn, &id) {
        Ok(gist) => {
            if gist.user_id != login_user.id {
                return Ok(Response::with(status::Forbidden));
            }
        }
        Err(e) => {
            error!("Errored: {:?}", e);
            return Ok(Response::with(status::InternalServerError));
        }
    }

    match models::gist::delete_by_id(&conn, &id) {
        Ok(_) => {
            return Ok(Response::with((status::Found, Redirect(helper::redirect_url("/gist/list")))));
        }
        Err(e) => {
            error!("Errored: {:?}", e);
            Ok(Response::with(status::InternalServerError))
        }
    }
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

    match models::gist::get_by_id(&conn, &id) {
        Ok(post_obj) => {
            mentions.push(post_obj.user.username);
        }
        Err(e) => {
            error!("Errored: {:?}", e);
            return Ok(Response::with(status::InternalServerError));
        }
    }

    match models::gist::get_comments_by_gist_id(&conn, &id) {
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


    match models::gist::add_comment(&conn, &login_id, &id, &body) {
        Ok(_) => {
            let title = String::from("New comment");
            let path = String::from("gist");
            helper::post_to_slack(&conn, &login_id, &title, &body, &id, mentions, &path);
            let url = Url::parse(&format!("{}/gist/show/{}", &CONFIG.team_domain, id)
                                     .to_string()).unwrap();
            return Ok(Response::with((status::Found, Redirect(url))));
        }
        Err(e) => {
            error!("Errored: {:?}", e);
            return Ok(Response::with(status::InternalServerError));
        }
    }
}

pub fn comment_update_handler(req: &mut Request) -> IronResult<Response> {
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
    let action: String;
    let body: String;
    let comment: models::gist::Comment;

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
            _ => return Ok(Response::with(status::BadRequest)),
        }

        match map.find(&["body"]) {
            Some(&Value::String(ref name)) => {
                body = name.to_string();
            }
            _ => return Ok(Response::with(status::BadRequest)),
        }
    }

    match models::gist::get_comment_by_id(&conn, &id) {
        Ok(db_comment) => {
            comment = db_comment;
            if comment.user_id != login_id {
                return Ok(Response::with(status::Forbidden));
            }
        }
        Err(e) => {
            error!("Errored: {:?}", e);
            return Ok(Response::with(status::InternalServerError));
        }
    }

    if action == "update" {
        match models::gist::update_comment_by_id(&conn, &id, &body) {
            Ok(_) => {
                let url = Url::parse(&format!("{}/gist/show/{}", &CONFIG.team_domain, comment.gist_id)
                                         .to_string()).unwrap();
                return Ok(Response::with((status::Found, Redirect(url))));
            }
            Err(e) => {
                error!("Errored: {:?}", e);
                return Ok(Response::with(status::InternalServerError));
            }
        }
    }
    if action == "delete" {
        match models::gist::delete_comment_by_id(&conn, &id) {
            Ok(_) => {
                let url = Url::parse(&format!("{}/gist/show/{}", &CONFIG.team_domain, comment.gist_id)
                                         .to_string()).unwrap();
                return Ok(Response::with((status::Found, Redirect(url))));
            }
            Err(e) => {
                error!("Errored: {:?}", e);
                return Ok(Response::with(status::InternalServerError));
            }
        }
    }
    return Ok(Response::with(status::InternalServerError));
}
