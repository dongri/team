use postgres::error::Error;
use db;

#[derive(Serialize, Debug, Default)]
pub struct User {
    pub id: i32,
    email: String,
    pub username: String,
    password: String,
}

pub fn create_user(conn: db::PostgresConnection, email: String, username: String, password: String) -> Result<(), Error> {
    conn.execute(
        "INSERT INTO users (email, username, password) VALUES ($1, $2, $3);",
        &[&email, &username, &password]
    ).map(|_| ())
}

pub fn get_user_by_username_password(conn: db::PostgresConnection, email: String, password: String) -> Result<User, Error> {
    let mut user: User = User{..Default::default()};
    for row in &conn.query("SELECT id, email, username, password from users where email = $1 and password = $2", &[&email, &password]).unwrap() {
        user = User {
            id: row.get("id"),
            email: row.get("email"),
            username: row.get("username"),
            password: row.get("password"),
        };
    }
    Ok(user)
}
