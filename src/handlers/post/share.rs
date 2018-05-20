use iron::prelude::*;
use iron::status;
use iron::Url;
use iron::modifiers::Redirect;
use db;
use persistent;
use router::Router;

use env::CONFIG;
use handlers;
use helper;
use models;

pub fn share_handler(req: &mut Request) -> IronResult<Response> {
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

    let ref id_str = req.extensions
        .get::<Router>()
        .unwrap()
        .find("id")
        .unwrap_or("/");
    let id = id_str.parse::<i32>().unwrap();

    match models::post::share_post(&conn, &id) {
        Ok(_) => {
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
