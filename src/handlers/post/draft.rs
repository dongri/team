use hbs::Template;
use hbs::handlebars::to_json;
use iron::prelude::*;
use iron::status;
use iron::modifiers::Redirect;
use db;
use persistent;

use handlers;
use helper;
use models;

pub fn draft_list_handler(req: &mut Request) -> IronResult<Response> {
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

    let mut resp = Response::new();

    #[derive(Serialize, Debug)]
    struct Data {
        logged_in: bool,
        login_user: models::user::User,
        posts: Vec<models::post::Post>,
    }

    let posts: Vec<models::post::Post>;

    match models::post::draft_list(&conn, &login_id) {
        Ok(posts_db) => {
            posts = posts_db;
        }
        Err(e) => {
            error!("Errored: {:?}", e);
            return Ok(Response::with(status::InternalServerError));
        }
    }
    let data = Data {
        logged_in: login_id != 0,
        login_user: login_user,
        posts: posts,
    };

    resp.set_mut(Template::new("draft/list", to_json(&data)))
        .set_mut(status::Ok);
    return Ok(resp);

}
