use iron::prelude::*;
use iron::status;
use hbs::{Template};
use std::collections::HashMap;

pub fn index_handler(_: &mut Request) -> IronResult<Response> {
    let mut resp = Response::new();
    let mut data = HashMap::new();
    data.insert(String::from("title"), "Team".to_string());
    resp.set_mut(Template::new("index", data)).set_mut(status::Ok);
    return Ok(resp);
}
