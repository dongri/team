use postgres::error::Error;
use chrono::{NaiveDateTime};
use db;
use helper;
use models;

#[derive(Serialize, Debug)]
pub struct Notification {
    pub id: i32,
    pub path: String,
    pub from_user: i32,
    pub to_user: i32,
    pub body: String,
    pub read: bool,
    pub created: NaiveDateTime,
    pub formated_created: String,
    pub user: models::user::User,
}

pub fn create(conn: &db::PostgresConnection, path: &String, from_user: &i32, to_user: &i32, body: &String) -> Result<(i32), Error> {
    let mut notification_id = 0;
    for row in &conn.query("
        INSERT INTO notifications (path, from_user, to_user, body)
        VALUES ($1, $2, $3, $4) returning id;",
        &[&path, &from_user, &to_user, &body]).unwrap() {
        notification_id = row.get("id");
    }
    Ok(notification_id)
}

pub fn list(conn: &db::PostgresConnection, to_user: &i32, offset: &i32, limit: &i32) -> Result<Vec<Notification>, Error> {
    conn.execute(
        "UPDATE notifications set read = true WHERE to_user = $1", &[&to_user]
    ).unwrap();
    let mut notifications: Vec<Notification> = Vec::new();
    for row in &conn.query("select n.*, u.id, u.username, u.icon_url from notifications as n join users as u on n.from_user = u.id 
        where n.to_user = $1::int 
        order by n.id desc 
        offset $2::int limit $3::int", &[&to_user, &offset, &limit]).unwrap() {
        let mut notification = Notification {
            id: row.get("id"),
            path: row.get("path"),
            from_user: row.get("from_user"),
            to_user: row.get("to_user"),
            body: row.get("body"),
            read: row.get("read"),
            created: row.get("created"),
            formated_created: "".to_string(),
            user: models::user::User{
                id: row.get("id"),
                username: row.get("username"),
                icon_url: row.get("icon_url"),
                username_hash: helper::username_hash(row.get("username")),
            }
        };
        notification.formated_created = helper::jst_time_formatter(notification.created);
        notifications.push(notification);
    }
    Ok(notifications)
}

pub fn count(conn: &db::PostgresConnection, to_user: &i32) -> Result<i32, Error> {
    let rows = &conn.query("SELECT count(*)::int as count from notifications where to_user = $1", &[&to_user]).unwrap();
    let row = rows.get(0);
    let count = row.get("count");
    Ok(count)
}

pub fn unread_count(conn: &db::PostgresConnection, to_user: &i32) -> Result<i32, Error> {
    let rows = &conn.query("SELECT count(*)::int as count from notifications where read = false and to_user = $1", &[&to_user]).unwrap();
    let row = rows.get(0);
    let count = row.get("count");
    Ok(count)
}