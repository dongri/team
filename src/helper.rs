use hbs::{Template};
use serde::ser::Serialize;
use crypto::sha2::Sha256;
use crypto::digest::Digest;

const SALT: &str = "6jpmgwMiTzFtFoF";

pub fn get_domain() -> String {
    return String::from("http://localhost:3000")
}

pub fn template<T: Serialize>(name: &str, data: T) -> Template {
    return Template::new(name, &data);
}

pub fn encrypt_password(plain_password: String) -> String {
    let mut sha256 = Sha256::new();
    sha256.input_str(&format!("{}{}", plain_password, SALT));
    return sha256.result_str();
}
