use chrono::NaiveDateTime;
use serde::Serialize;

use crate::db::PgPool;
use crate::models;
use crate::util;

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

pub async fn create(
    pool: &PgPool,
    path: &str,
    from_user: i32,
    to_user: i32,
    body: &str,
) -> anyhow::Result<i32> {
    let conn = pool.get().await?;
    let row = conn
        .query_one(
            "INSERT INTO notifications (path, from_user, to_user, body)
             VALUES ($1, $2, $3, $4) returning id;",
            &[&path, &from_user, &to_user, &body],
        )
        .await?;
    Ok(row.get("id"))
}

pub async fn list(
    pool: &PgPool,
    to_user: i32,
    offset: i32,
    limit: i32,
) -> anyhow::Result<Vec<Notification>> {
    let conn = pool.get().await?;
    conn.execute(
        "UPDATE notifications set read = true WHERE to_user = $1",
        &[&to_user],
    )
    .await?;
    let mut notifications: Vec<Notification> = Vec::new();
    for row in conn
        .query(
            "select n.*, u.id, u.username, u.icon_url from notifications as n
             join users as u on n.from_user = u.id
             where n.to_user = $1::int
             order by n.id desc
             offset $2::int limit $3::int",
            &[&to_user, &offset, &limit],
        )
        .await?
    {
        let username: String = row.get("username");
        let created: NaiveDateTime = row.get("created");
        notifications.push(Notification {
            id: row.get("id"),
            path: row.get("path"),
            from_user: row.get("from_user"),
            to_user: row.get("to_user"),
            body: row.get("body"),
            read: row.get("read"),
            created,
            formated_created: util::jst_time_formatter(created),
            user: models::user::User {
                id: row.get("from_user"),
                username_hash: util::username_hash(&username),
                username,
                icon_url: row.get("icon_url"),
            },
        });
    }
    Ok(notifications)
}

pub async fn count(pool: &PgPool, to_user: i32) -> anyhow::Result<i32> {
    let conn = pool.get().await?;
    let row = conn
        .query_one(
            "SELECT count(*)::int as count from notifications where to_user = $1",
            &[&to_user],
        )
        .await?;
    Ok(row.get("count"))
}

pub async fn unread_count(pool: &PgPool, to_user: i32) -> anyhow::Result<i32> {
    let conn = pool.get().await?;
    let row = conn
        .query_one(
            "SELECT count(*)::int as count from notifications where read = false and to_user = $1",
            &[&to_user],
        )
        .await?;
    Ok(row.get("count"))
}
