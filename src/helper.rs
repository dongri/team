use hbs::{Template};
use serde::ser::Serialize;

pub fn get_domain() -> String {
    return String::from("http://localhost:3000")
}

pub fn template<T: Serialize>(name: &str, data: T) -> Template {
    return Template::new(name, &data);
}
