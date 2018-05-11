use postgres::error::Error;
use db;
use helper;
use env;

#[derive(Serialize, Debug, Default)]
pub struct User {
    pub id: i32,
    pub username: String,
    pub icon_url: Option<String>,
    pub username_hash: String,
}

#[derive(Serialize, Debug, Default)]
pub struct UserWithPassword {
    pub id: i32,
    pub username: String,
    pub icon_url: Option<String>,
    pub username_hash: String,
    pub password: String,
}

#[derive(Serialize, Debug, Default)]
pub struct UserWithEmail {
    pub id: i32,
    pub username: String,
    pub icon_url: Option<String>,
    pub email: Option<String>,
    pub username_hash: String,
}

#[derive(Serialize, Debug, Default)]
pub struct UserWithPreference {
    pub id: i32,
    pub username: String,
    pub icon_url: Option<String>,
    pub username_hash: String,
    pub menu: Vec<String>,
    pub theme: String,
}

pub fn create(conn: &db::PostgresConnection, username: &String, password: &String) -> Result<(i32), Error> {
    let rows = &conn.query("INSERT INTO users (username, password) VALUES ($1, $2) returning id;", &[&username, &password]).unwrap();
    let row = rows.get(0);
    let user_id: i32 = row.get("id");
    Ok(user_id)
}

pub fn create_with_email(conn: &db::PostgresConnection, username: &String, email: &String) -> Result<(i32), Error> {
    let rows = &conn.query("INSERT INTO users (username, email) VALUES ($1, $2) returning id;", &[&username, &email]).unwrap();
    let row = rows.get(0);
    let user_id: i32 = row.get("id");
    Ok(user_id)
}

pub fn get_by_username_password(conn: &db::PostgresConnection, username: &String, password: &String) -> Result<User, Error> {
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

pub fn get_by_id(conn: &db::PostgresConnection, id: &i32) -> Result<User, Error> {
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

pub fn get_current_user(conn: &db::PostgresConnection, id: &i32) -> Result<UserWithPreference, Error> {
    let mut user: UserWithPreference = UserWithPreference{..Default::default()};
    let default_menu = &env::CONFIG.team_menu;
    let default_theme = &env::CONFIG.team_theme;
    for row in &conn.query("SELECT u.id, u.username, u.icon_url, COALESCE(p.menu, $2) as menu, COALESCE(p.theme, $3) as theme from users as u left join preferences as p on u.id=p.user_id where u.id = $1", &[&id, &default_menu, &default_theme]).unwrap() {
        user = UserWithPreference {
            id: row.get("id"),
            username: row.get("username"),
            icon_url: row.get("icon_url"),
            username_hash: helper::username_hash(row.get("username")),
            menu: helper::split_menu(row.get("menu")),
            theme: row.get("theme"),
        };
    }
    Ok(user)
}

pub fn get_with_password_by_id(conn: &db::PostgresConnection, id: &i32) -> Result<UserWithPassword, Error> {
    let mut user = UserWithPassword{..Default::default()};
    for row in &conn.query("SELECT id, username, icon_url, password from users where id = $1", &[&id]).unwrap() {
        user = UserWithPassword {
            id: row.get("id"),
            username: row.get("username"),
            icon_url: row.get("icon_url"),
            username_hash: helper::username_hash(row.get("username")),
            password: row.get("password"),
        };
    }
    Ok(user)
}

pub fn update_icon_url(conn: &db::PostgresConnection, id: &i32, icon_url: &String) -> Result<(), Error>{
    conn.execute(
        "UPDATE users set icon_url = $2 WHERE id = $1", &[&id, &icon_url]
    ).map(|_| ())
}

pub fn update_password(conn: &db::PostgresConnection, id: &i32, password: &String) -> Result<(), Error>{
    conn.execute(
        "UPDATE users set password = $2 WHERE id = $1", &[&id, &password]
    ).map(|_| ())
}

pub fn update_username(conn: &db::PostgresConnection, id: &i32, username: &String) -> Result<(), Error>{
    conn.execute(
        "UPDATE users set username = $2 WHERE id = $1", &[&id, &username]
    ).map(|_| ())
}

pub fn get_by_username(conn: &db::PostgresConnection, username: &str) -> Result<User, Error> {
    let mut user: User = User{..Default::default()};
    for row in &conn.query("SELECT id, username, icon_url from users where username = $1", &[&username]).unwrap() {
        user = User {
            id: row.get("id"),
            username: row.get("username"),
            icon_url: row.get("icon_url"),
            username_hash: helper::username_hash(row.get("username")),
        };
    }
    Ok(user)
}

pub fn get_by_email(conn: &db::PostgresConnection, email: &str) -> Result<UserWithEmail, Error> {
    let mut user: UserWithEmail = UserWithEmail{..Default::default()};
    for row in &conn.query("SELECT id, username, icon_url, email from users where email = $1", &[&email]).unwrap() {
        user = UserWithEmail {
            id: row.get("id"),
            username: row.get("username"),
            icon_url: row.get("icon_url"),
            email: row.get("email"),
            username_hash: helper::username_hash(row.get("username")),
        };
    }
    Ok(user)
}

pub fn set_preference_menu(conn: &db::PostgresConnection, user_id: &i32, menu: &String) -> Result<(), Error>{
    conn.execute(
        "update preferences set menu=$2 where user_id=$1", &[&user_id, &menu]
    ).unwrap();
    conn.execute(
        "insert into preferences (user_id, menu) 
        select $1, $2
        where not exists (select 1 from preferences where user_id=$1)", &[&user_id, &menu]
    ).map(|_| ())
}

pub fn set_preference_theme(conn: &db::PostgresConnection, user_id: &i32, theme: &String) -> Result<(), Error>{
    conn.execute(
        "update preferences set theme=$2 where user_id=$1", &[&user_id, &theme]
    ).unwrap();
    conn.execute(
        "insert into preferences (user_id, theme) 
        select $1, $2
        where not exists (select 1 from preferences where user_id=$1)", &[&user_id, &theme]
    ).map(|_| ())
}
