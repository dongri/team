pub mod account;
pub mod gist;
pub mod index;
pub mod post;
pub mod router;
pub mod tweet;

use serde::Deserialize;

/// Common `?page=N` query parameter.
#[derive(Deserialize, Debug, Default)]
pub struct PageQuery {
    pub page: Option<i32>,
}
