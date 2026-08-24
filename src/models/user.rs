use serde::Serialize;

use crate::config::Config;
use crate::db::PgPool;
use crate::util;

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

#[derive(Serialize, Debug, Default, Clone)]
pub struct UserWithPreference {
    pub id: i32,
    pub username: String,
    pub icon_url: Option<String>,
    pub username_hash: String,
    pub menu: Vec<String>,
    pub theme: String,
}

fn user_from_row(row: &tokio_postgres::Row) -> User {
    let username: String = row.get("username");
    User {
        id: row.get("id"),
        username_hash: util::username_hash(&username),
        username,
        icon_url: row.get("icon_url"),
    }
}

pub async fn create(pool: &PgPool, username: &str, password: &str) -> anyhow::Result<i32> {
    let conn = pool.get().await?;
    let row = conn
        .query_one(
            "INSERT INTO users (username, password) VALUES ($1, $2) returning id;",
            &[&username, &password],
        )
        .await?;
    Ok(row.get("id"))
}

pub async fn create_with_email(pool: &PgPool, username: &str, email: &str) -> anyhow::Result<i32> {
    let conn = pool.get().await?;
    let row = conn
        .query_one(
            "INSERT INTO users (username, email) VALUES ($1, $2) returning id;",
            &[&username, &email],
        )
        .await?;
    Ok(row.get("id"))
}

pub async fn get_with_password_by_username(
    pool: &PgPool,
    username: &str,
) -> anyhow::Result<UserWithPassword> {
    let conn = pool.get().await?;
    let mut user = UserWithPassword::default();
    for row in conn
        .query(
            "SELECT id, username, icon_url, password from users where username = $1",
            &[&username],
        )
        .await?
    {
        let name: String = row.get("username");
        user = UserWithPassword {
            id: row.get("id"),
            username_hash: util::username_hash(&name),
            username: name,
            icon_url: row.get("icon_url"),
            password: row.get::<_, Option<String>>("password").unwrap_or_default(),
        };
    }
    Ok(user)
}

pub async fn get_by_id(pool: &PgPool, id: i32) -> anyhow::Result<User> {
    let conn = pool.get().await?;
    let mut user = User::default();
    for row in conn
        .query(
            "SELECT id, username, icon_url from users where id = $1",
            &[&id],
        )
        .await?
    {
        user = user_from_row(&row);
    }
    Ok(user)
}

pub async fn get_current_user(
    pool: &PgPool,
    id: i32,
    config: &Config,
) -> anyhow::Result<UserWithPreference> {
    let conn = pool.get().await?;
    let mut user = UserWithPreference::default();
    for row in conn
        .query(
            "SELECT u.id, u.username, u.icon_url, COALESCE(p.menu, $2) as menu, COALESCE(p.theme, $3) as theme
             from users as u
             left join preferences as p on u.id = p.user_id
             where u.id = $1",
            &[&id, &config.team_menu, &config.team_theme],
        )
        .await?
    {
        let username: String = row.get("username");
        let menu: String = row.get("menu");
        user = UserWithPreference {
            id: row.get("id"),
            username_hash: util::username_hash(&username),
            username,
            icon_url: row.get("icon_url"),
            menu: util::split_menu(&menu),
            theme: row.get("theme"),
        };
    }
    Ok(user)
}

pub async fn get_with_password_by_id(pool: &PgPool, id: i32) -> anyhow::Result<UserWithPassword> {
    let conn = pool.get().await?;
    let mut user = UserWithPassword::default();
    for row in conn
        .query(
            "SELECT id, username, icon_url, password from users where id = $1",
            &[&id],
        )
        .await?
    {
        let username: String = row.get("username");
        user = UserWithPassword {
            id: row.get("id"),
            username_hash: util::username_hash(&username),
            username,
            icon_url: row.get("icon_url"),
            // NULL for accounts created via Google sign-in
            password: row.get::<_, Option<String>>("password").unwrap_or_default(),
        };
    }
    Ok(user)
}

pub async fn update_icon_url(pool: &PgPool, id: i32, icon_url: &str) -> anyhow::Result<()> {
    let conn = pool.get().await?;
    conn.execute(
        "UPDATE users set icon_url = $2 WHERE id = $1",
        &[&id, &icon_url],
    )
    .await?;
    Ok(())
}

pub async fn update_password(pool: &PgPool, id: i32, password: &str) -> anyhow::Result<()> {
    let conn = pool.get().await?;
    conn.execute(
        "UPDATE users set password = $2 WHERE id = $1",
        &[&id, &password],
    )
    .await?;
    Ok(())
}

pub async fn update_username(pool: &PgPool, id: i32, username: &str) -> anyhow::Result<()> {
    let conn = pool.get().await?;
    conn.execute(
        "UPDATE users set username = $2 WHERE id = $1",
        &[&id, &username],
    )
    .await?;
    Ok(())
}

pub async fn get_by_username(pool: &PgPool, username: &str) -> anyhow::Result<User> {
    let conn = pool.get().await?;
    let mut user = User::default();
    for row in conn
        .query(
            "SELECT id, username, icon_url from users where username = $1",
            &[&username],
        )
        .await?
    {
        user = user_from_row(&row);
    }
    Ok(user)
}

pub async fn get_by_email(pool: &PgPool, email: &str) -> anyhow::Result<UserWithEmail> {
    let conn = pool.get().await?;
    let mut user = UserWithEmail::default();
    for row in conn
        .query(
            "SELECT id, username, icon_url, email from users where email = $1",
            &[&email],
        )
        .await?
    {
        let username: String = row.get("username");
        user = UserWithEmail {
            id: row.get("id"),
            username_hash: util::username_hash(&username),
            username,
            icon_url: row.get("icon_url"),
            email: row.get("email"),
        };
    }
    Ok(user)
}

pub async fn set_preference_menu(pool: &PgPool, user_id: i32, menu: &str) -> anyhow::Result<()> {
    let conn = pool.get().await?;
    conn.execute(
        "update preferences set menu=$2 where user_id=$1",
        &[&user_id, &menu],
    )
    .await?;
    conn.execute(
        "insert into preferences (user_id, menu)
         select $1, $2
         where not exists (select 1 from preferences where user_id=$1)",
        &[&user_id, &menu],
    )
    .await?;
    Ok(())
}

pub async fn set_preference_theme(pool: &PgPool, user_id: i32, theme: &str) -> anyhow::Result<()> {
    let conn = pool.get().await?;
    conn.execute(
        "update preferences set theme=$2 where user_id=$1",
        &[&user_id, &theme],
    )
    .await?;
    conn.execute(
        "insert into preferences (user_id, theme)
         select $1, $2
         where not exists (select 1 from preferences where user_id=$1)",
        &[&user_id, &theme],
    )
    .await?;
    Ok(())
}
