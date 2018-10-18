use std::error::Error;

use hbs::{DirectorySource, HandlebarsEngine};
use iron::prelude::*;
use iron::middleware::{AroundMiddleware, Handler};
use iron_sessionstorage::SessionStorage;
use iron_sessionstorage::backends::SignedCookieBackend;
use time;

use env::CONFIG;

struct Logger;
impl Logger {
    fn log(&self, req: &Request, res: Result<&Response, &IronError>, time: u64) {
        info!(
            "Request: {:?}\nResponse: {:?}\nResponse-Time: {:?}",
            req, res, time
        )
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

pub fn setup<H: Handler>(handler: H) -> Chain {
    let mut chain = Chain::new(handler);

    let mut hbse = HandlebarsEngine::new();
    hbse.add(Box::new(DirectorySource::new("./templates/", ".hbs")));
    if let Err(r) = hbse.reload() {
        panic!("{}", r.description());
    }
    chain.link_after(hbse);

    let secret = &CONFIG.team_cookie_secret.as_bytes();
    chain.link_around(SessionStorage::new(SignedCookieBackend::new(secret.to_vec())));

    chain.around(Logger);

    return chain;
}
