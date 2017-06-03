extern crate iron;
#[macro_use]
extern crate router;
extern crate handlebars_iron as hbs;
extern crate params;
extern crate staticfile;
extern crate mount;
extern crate persistent;
extern crate iron_sessionstorage;
extern crate urlencoded;

extern crate postgres;
extern crate r2d2;
extern crate r2d2_postgres;

extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;

extern crate crypto;

extern crate slack_hook;

use std::error::Error;
use std::path::Path;
use std::env;

use iron::prelude::*;
use router::{Router};
use hbs::{HandlebarsEngine, DirectorySource};
use staticfile::Static;
use mount::Mount;
use persistent::Read as PRead;

use iron_sessionstorage::SessionStorage;
use iron_sessionstorage::backends::SignedCookieBackend;

#[macro_use]
mod db;
mod handlers;
mod models;
mod helper;

fn main() {
    let router = handlers::router::create_router();

    let mut chain = Chain::new(router);

    let mut hbse = HandlebarsEngine::new();
    hbse.add(Box::new(
        DirectorySource::new("./templates/", ".hbs")));
    if let Err(r) = hbse.reload() {
        panic!("{}", r.description());
    }
    chain.link_after(hbse);

    let conn_string:String = match env::var("TEAM_DATABASE_URL") {
        Ok(val) => val,
        Err(_) => "postgres://root:@localhost:5432/team".to_string()
    };

    let pool = db::get_pool(&conn_string);
    chain.link(PRead::<db::PostgresDB>::both(pool));

    let secret = b"FLEo9NZJDhZbBaT".to_vec();
    chain.link_around(SessionStorage::new(SignedCookieBackend::new(secret)));

    let mut mount = Mount::new();
    mount.mount("/css", Static::new(Path::new("./public/css/")));
    mount.mount("/js", Static::new(Path::new("./public/js/")));
    mount.mount("/img", Static::new(Path::new("./public/img/")));
    mount.mount("/", chain);

    println!("Listen on localhost:3000");
    Iron::new(mount).http("localhost:3000").unwrap();
}
