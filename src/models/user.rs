use postgres::error::Error;
use db;
use helper;

#[derive(Serialize, Debug, Default)]
pub struct User {
    pub id: i32,
    pub username: String,
    pub icon_url: Option<String>,
    pub username_hash: String,
}

pub fn create(conn: db::PostgresConnection, username: String, password: String) -> Result<(), Error> {
    conn.execute(
        "INSERT INTO users (username, password) VALUES ($1, $2);",
        &[&username, &password]
    ).map(|_| ())
}

pub fn get_by_username_password(conn: db::PostgresConnection, username: String, password: String) -> Result<User, Error> {
    let mut user: User = User{..Default::default()};
    for row in &conn.query("SELECT id, username, icon_url from users where username = $1 and password = $2", &[&username, &password]).unwrap() {
        user = User {
            id: row.get("id"),
            username: row.get("username"),
            icon_url: row.get("icon_url"),
            username_hash: helper::username_hash(row.get("username")),
        };
    }
    Ok(user)
}

pub fn get_by_id(conn: db::PostgresConnection, id: i32) -> Result<User, Error> {
    let mut user: User = User{..Default::default()};
    for row in &conn.query("SELECT id, username, icon_url from users where id = $1", &[&id]).unwrap() {
        user = User {
            id: row.get("id"),
            username: row.get("username"),
            icon_url: row.get("icon_url"),
            username_hash: helper::username_hash(row.get("username")),
        };
    }
    Ok(user)
}

pub fn update_icon_url(conn: db::PostgresConnection, id: i32, icon_url: String) -> Result<(), Error>{
    conn.execute(
        "UPDATE users set icon_url = $2 WHERE id = $1", &[&id, &icon_url]
    ).map(|_| ())
}
