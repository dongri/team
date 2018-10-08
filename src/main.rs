extern crate handlebars_iron as hbs;
extern crate iron;
extern crate iron_sessionstorage;
extern crate mount;
extern crate params;
extern crate persistent;
extern crate router;
extern crate staticfile;
extern crate urlencoded;

extern crate postgres;
extern crate r2d2;
extern crate r2d2_postgres;

extern crate envy;
extern crate serde;
#[macro_use]
extern crate serde_json;

#[macro_use]
extern crate serde_derive;

extern crate crypto;

extern crate slack_hook;

extern crate chrono;
extern crate time;

extern crate diff;

extern crate futures;
extern crate hyper;
extern crate hyper_tls;
extern crate tokio_core;

#[macro_use]
extern crate lazy_static;

extern crate fern;
#[macro_use]
extern crate log;

extern crate oauth2;
extern crate reqwest;
extern crate url;

use iron::prelude::*;
use router::Router;
use persistent::Read as PRead;

#[macro_use]
mod db;
mod handlers;
mod models;
mod helper;
mod env;
mod middlewares;

fn setup_fern(level: log::LogLevelFilter, verbose: bool) {
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{}][{}/{}:{}][{}] {}",
                chrono::Local::now().to_rfc3339(),
                record.location().module_path(),
                record.location().file(),
                record.location().line(),
                record.level(),
                message
            ))
        })
        .level(level)
        .chain(std::io::stdout())
        .filter(move |meta: &log::LogMetadata| verbose || meta.target().starts_with("team"))
        .apply()
        .unwrap()
}

fn main() {
    setup_fern(log::LogLevelFilter::Debug, false);
    let mount = handlers::router::mount_path();
    let mut chain = middlewares::setup(mount);

    match db::get_pool(&env::CONFIG.team_database_url.as_str()) {
        Ok(pool) => chain.link(PRead::<db::PostgresDB>::both(pool)),
        Err(err) => {
            error!("postgres: {}", err);
            std::process::exit(-1);
        }
    };

    let listen = format!("{}:{}", "0.0.0.0", &env::CONFIG.port);
    info!("Listen {:?}", listen);
    Iron::new(chain).http(listen).unwrap();
}
