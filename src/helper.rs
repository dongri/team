use hbs::Template;
use serde::ser::Serialize;
use crypto::sha2::Sha256;
use crypto::digest::Digest;
use slack_hook::{Slack, PayloadBuilder};
use std::env;

// hyper
use std::io::{self, Write};
use futures::{Future, Stream};
use hyper::Client;
use tokio_core::reactor::Core;

use hyper::{Method, Request};
use hyper::header::{ContentLength, ContentType};

use db;
use models;

const SALT: &str = "6jpmgwMiTzFtFoF";

pub fn get_env(key: &str) -> String {
    let value: String = match env::var(key) {
        Ok(val) => val,
        Err(_) => "".to_string(),
    };
    return value;
}

pub fn get_domain() -> String {
    let domain = get_env("TEAM_DOMAIN");
    return domain;
}

pub fn get_webhook_url() -> String {
    let url = get_env("TEAM_WEBHOOK_URL");
    return url;
}

pub fn template<T: Serialize>(name: &str, data: T) -> Template {
    return Template::new(name, &data);
}

pub fn encrypt_password(plain_password: String) -> String {
    let mut sha256 = Sha256::new();
    sha256.input_str(&format!("{}{}", plain_password, SALT));
    return sha256.result_str();
}

pub fn username_hash(username: String) -> String {
    let mut sha256 = Sha256::new();
    sha256.input_str(&format!("{}", username));
    return sha256.result_str();
}

pub fn post_to_slack(conn: &db::PostgresConnection, user_id: &i32, title: &String, body: &String, post_id: &i32) {
    match models::user::get_by_id(&conn, &user_id) {
        Ok(user) => {
            let link = format!("{}/{}/{}", get_domain(), "post/show", post_id).to_string();
            let text = format!("{} by @{}\n{}\n{}", title, user.username, body, link).to_string();
            slack(text);
        }
        Err(e) => {
            println!("Errored: {:?}", e);
        }
    }
}

pub fn slack(text: String) {
    let slack_url = get_env("TEAM_SLACK");
    let url = slack_url.as_str();
    let slack = Slack::new(url);
    match slack {
        Ok(slack) => {
            let p = PayloadBuilder::new()
                .text(text)
                //.channel("#team")
                .username("Team")
                .icon_emoji(":beers:")
                .build()
                .unwrap();
            let res = slack.send(&p);
            println!("{:?}", res);
        }
        _ => println!("can not connect to slack(env TEAM_SLACK={})", url),
    }
}

pub fn webhook(username: String, title: String, body: String, url: String) {
    let webhookURL = get_webhook_url();
    if (webhookURL == "") {
        return
    }

    let mut core = Core::new().unwrap();
    let handle = core.handle();
    let client = Client::new(&handle);

    let b = format!(r#"{:?}"#, body);
    let json = format!(r#"{{"username": "{}", "title": "{}", "body": {}, "url": "{}"}}"#, username, title, b, url);

    let uri = webhookURL.parse().unwrap();
    let mut req = Request::new(Method::Post, uri);
    req.headers_mut().set(ContentType::json());
    req.headers_mut().set(ContentLength(json.len() as u64));
    req.set_body(json);

    let post = client.request(req).and_then(|res| {
        println!("POST: {}", res.status());
        //res.body().concat2()
        res.body().for_each(|chunk| {
            io::stdout()
                .write_all(&chunk)
                .map(|_| ())
                .map_err(From::from)
            })
    });

    core.run(post);
}


use iron::status;
use params::{Map, Value};
pub fn get_param(map: &Map, name: &str) -> Result<String, status::Status> {
    match map.get(name) {
        Some(&Value::String(ref value)) => {
            return Ok(value.to_string());
        }
        _ => return Err(status::BadRequest),
    }
}
