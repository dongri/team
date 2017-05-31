use postgres::error::Error;
use db;

#[derive(Serialize, Debug, Default)]
pub struct User {
    pub id: i32,
    pub email: String,
    pub username: String,
    pub icon_url: Option<String>,
}

pub fn create(conn: db::PostgresConnection, email: String, username: String, password: String) -> Result<(), Error> {
    conn.execute(
        "INSERT INTO users (email, username, password) VALUES ($1, $2, $3);",
        &[&email, &username, &password]
    ).map(|_| ())
}

pub fn get_by_username_password(conn: db::PostgresConnection, email: String, password: String) -> Result<User, Error> {
    let mut user: User = User{..Default::default()};
    for row in &conn.query("SELECT id, email, username, icon_url from users where email = $1 and password = $2", &[&email, &password]).unwrap() {
        user = User {
            id: row.get("id"),
            email: row.get("email"),
            username: row.get("username"),
            icon_url: row.get("icon_url"),
        };
    }
    Ok(user)
}

pub fn get_by_id(conn: db::PostgresConnection, id: i32) -> Result<User, Error> {
    let mut user: User = User{..Default::default()};
    for row in &conn.query("SELECT id, email, username, icon_url from users where id = $1", &[&id]).unwrap() {
        user = User {
            id: row.get("id"),
            email: row.get("email"),
            username: row.get("username"),
            icon_url: row.get("icon_url"),
        };
    }
    Ok(user)
}

pub fn update_icon_url(conn: db::PostgresConnection, id: i32, icon_url: String) -> Result<(), Error>{
    conn.execute(
        "UPDATE users set icon_url = $2 WHERE id = $1", &[&id, &icon_url]
    ).map(|_| ())
}
