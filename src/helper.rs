use hbs::{Template};
use serde::ser::Serialize;
use crypto::sha2::Sha256;
use crypto::digest::Digest;
use slack_hook::{Slack, PayloadBuilder};
use std::env;
use std::mem;

const SALT: &str = "6jpmgwMiTzFtFoF";

pub fn get_env(key: &str) -> String {
    let value: String = match env::var(key) {
        Ok(val) => val,
        Err(_) => "".to_string()
    };
    return value
}

pub fn get_domain() -> String {
    let domain = get_env("TEAM_DOMAIN");
    return domain
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

pub fn slack(text: String) {
    let slack_url: String = get_env("TEAM_SLACK");
    let url: &'static str = string_to_static_str(slack_url);
    let slack = Slack::new(url).unwrap();
    let p = PayloadBuilder::new()
      .text(text)
      .channel("#team")
      .username("Team")
      .icon_emoji(":beers:")
      .build()
      .unwrap();
    let res = slack.send(&p);
    println!("{:?}", res);
}

pub fn string_to_static_str(s: String) -> &'static str {
    unsafe {
        let ret = mem::transmute(&s as &str);
        mem::forget(s);
        ret
    }
}
