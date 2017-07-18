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

extern crate time;
extern crate chrono;

extern crate diff;

extern crate hyper;
extern crate hyper_tls;
extern crate futures;
extern crate tokio_core;

#[macro_use]
extern crate log;
extern crate fern;

use std::error::Error;
use std::path::Path;

use iron::prelude::*;
use router::Router;
use hbs::{HandlebarsEngine, DirectorySource};
use staticfile::Static;
use mount::Mount;
use persistent::Read as PRead;

use iron_sessionstorage::SessionStorage;
use iron_sessionstorage::backends::SignedCookieBackend;
use iron::middleware::{AroundMiddleware, Handler};

#[macro_use]
mod db;
mod handlers;
mod models;
mod helper;

struct LoggerHandler<H: Handler> {
    logger: Logger,
    handler: H,
}
impl<H: Handler> Handler for LoggerHandler<H> {
    fn handle(&self, req: &mut Request) -> IronResult<Response> {
        let entry = time::precise_time_ns();
        let res = self.handler.handle(req);
        let time = time::precise_time_ns() - entry;
        self.logger.log(req, res.as_ref(), time);
        res
    }
}
struct Logger;
impl Logger {
    fn log(&self, req: &Request, res: Result<&Response, &IronError>, time: u64) {
        info!("Request: {:?}\nResponse: {:?}\nResponse-Time: {:?}",
              req,
              res,
              time)
    }
}
impl AroundMiddleware for Logger {
    fn around(self, handler: Box<Handler>) -> Box<Handler> {
        Box::new(LoggerHandler {
                     logger: self,
                     handler: handler,
                 }) as Box<Handler>
    }
}

fn setup_fern(level: log::LogLevelFilter, verbose: bool) {
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!("[{}][{}/{}:{}][{}] {}",
                                    chrono::Local::now().to_rfc3339(),
                                    record.location().module_path(),
                                    record.location().file(),
                                    record.location().line(),
                                    record.level(),
                                    message))
        })
        .level(level)
        .chain(std::io::stdout())
        .filter(move |meta: &log::LogMetadata| verbose || meta.target() == "team")
        .apply()
        .unwrap()
}

fn main() {
    setup_fern(log::LogLevelFilter::Debug, false);
    let router = handlers::router::create_router();

    let mut mount = Mount::new();
    mount.mount("/css", Static::new(Path::new("./public/css/")));
    mount.mount("/js", Static::new(Path::new("./public/js/")));
    mount.mount("/img", Static::new(Path::new("./public/img/")));
    mount.mount("/fonts", Static::new(Path::new("./public/fonts/")));
    mount.mount("/", router);

    let mut chain = Chain::new(mount);

    let mut hbse = HandlebarsEngine::new();
    hbse.add(Box::new(DirectorySource::new("./templates/", ".hbs")));
    if let Err(r) = hbse.reload() {
        panic!("{}", r.description());
    }
    chain.link_after(hbse);

    let conn_string = helper::get_env("TEAM_DATABASE_URL");
    match db::get_pool(&conn_string) {
        Ok(pool) => chain.link(PRead::<db::PostgresDB>::both(pool)),
        Err(err) => {
            error!("postgres: {}", err);
            std::process::exit(-1);
        },
    };

    let secret = b"FLEo9NZJDhZbBaT".to_vec();
    chain.link_around(SessionStorage::new(SignedCookieBackend::new(secret)));

    chain.around(Logger);

    let mut port = helper::get_env("PORT");
    if port == "" {
        port = "3000".to_string();
    }
    let listen = format!("{}:{}", "0.0.0.0", port);
    info!("Listen {:?}", listen);
    Iron::new(chain).http(listen).unwrap();
}
