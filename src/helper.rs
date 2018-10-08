use hbs::Template;
use serde::ser::Serialize;
use crypto::sha2::Sha256;
use crypto::digest::Digest;
use slack_hook::{Slack, PayloadBuilder};
use chrono::{NaiveDateTime};
use time::Duration;

// hyper
use hyper::Client;
use hyper_tls::HttpsConnector;
use tokio_core::reactor::Core;

use hyper::{Method, Request};
use hyper::header::{ContentLength, ContentType};

use db;
use env::CONFIG;
use models;

const SALT: &str = "6jpmgwMiTzFtFoF";

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

pub fn post_to_slack(conn: &db::PostgresConnection, user_id: &i32, title: &String, body: &String, post_id: &i32, mentions: Vec<String>, path: &String) {
    match models::user::get_by_id(&conn, &user_id) {
        Ok(user) => {
            let link = format!("{}/{}/{}/{}", &CONFIG.team_domain, path, "show", post_id).to_string();
            let mut mentions_str: String = "".to_owned();
            for m in mentions {
                mentions_str.push_str(&"@".to_owned());
                mentions_str.push_str(&m.to_owned());
                mentions_str.push_str(&" ".to_owned());
            }
            let text = format!("{} by @{}\n{}\n{}\n{}", title, user.username, body, link, mentions_str).to_string();
            slack(text);
        }
        Err(e) => {
            error!("Errored: {:?}", e);
        }
    }
}

pub fn slack(text: String) {
    let slack_url = &CONFIG.team_slack;
    if slack_url == "" {
        debug!("TEAM_SLACK: {}", text);
        return;
    }
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
            error!("{:?}", res);
        }
        _ => error!("can not connect to slack(env TEAM_SLACK={})", url),
    }
}

pub fn webhook(username: String, title: String, body: String, url: String) {
    let webhook_url = &CONFIG.team_webhook_url;
    if webhook_url == "" {
        return
    }

    let mut core = Core::new().unwrap();
    let handle = core.handle();
    let client = Client::configure()
        .connector(HttpsConnector::new(4, &handle).unwrap())
        .build(&handle);

    let data = json!({
        "username": username,
        "title": title,
        "body": body,
        "url": url,
    });

    let json_string = data.to_string();

    let uri = webhook_url.parse().unwrap();
    let mut req = Request::new(Method::Post, uri);
    req.headers_mut().set(ContentType::json());
    req.headers_mut().set(ContentLength(json_string.len() as u64));
    req.set_body(json_string);

    let post = client.request(req);
    let res = core.run(post);
    error!("{:?}", res);
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

use iron::Url;
pub fn redirect_url(path: &str) -> Url {
    let url = Url::parse(&format!("{}{}", &CONFIG.team_domain, path)
            .to_string())
            .unwrap();
    return url
}

// use std;
use reqwest;

pub fn get_google_email(access_token: String) -> String {
    #[derive(Deserialize, Default, Debug)]
    struct Info {
        email: String,
    }
    let url = &format!("{}{}","https://www.googleapis.com/oauth2/v1/tokeninfo?access_token=", access_token).to_string();
    let mut res = reqwest::get(url).unwrap();
    if let Ok(info) = res.json::<Info>() {
        return info.email
    } else {
        return "".to_string()
    }
}

pub fn uppercase_first_letter(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

pub fn jst_time_formatter(time: NaiveDateTime) -> String {
    return (time + Duration::hours(9)).format("%Y-%m-%d %H:%M:%S").to_string()
}

pub fn split_menu(menu: String) -> Vec<String> {
    return menu.split(",").map(|s| s.to_string()).collect();
}
