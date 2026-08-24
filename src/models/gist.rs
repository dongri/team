use chrono::NaiveDateTime;
use serde::Serialize;
use tokio_postgres::Row;

use crate::db::PgPool;
use crate::models;
use crate::util;

#[derive(Serialize, Debug)]
pub struct Gist {
    pub id: i32,
    pub user_id: i32,
    pub description: String,
    pub filename: String,
    pub code: String,
    pub created: NaiveDateTime,
    pub formated_created: String,
    pub user: models::user::User,
}

fn gist_from_row(row: &Row) -> Gist {
    let username: String = row.get("username");
    let created: NaiveDateTime = row.get("created");
    Gist {
        id: row.get("id"),
        user_id: row.get("user_id"),
        description: row.get("description"),
        filename: row.get("filename"),
        code: row.get("code"),
        created,
        formated_created: util::jst_time_formatter(created),
        user: models::user::User {
            id: row.get("user_id"),
            username_hash: util::username_hash(&username),
            username,
            icon_url: row.get("icon_url"),
        },
    }
}

pub async fn create(
    pool: &PgPool,
    user_id: i32,
    description: &str,
    filename: &str,
    code: &str,
) -> anyhow::Result<i32> {
    let conn = pool.get().await?;
    let row = conn
        .query_one(
            "INSERT INTO gists (user_id, description, filename, code)
             VALUES ($1, $2, $3, $4) returning id;",
            &[&user_id, &description, &filename, &code],
        )
        .await?;
    Ok(row.get("id"))
}

pub async fn list(pool: &PgPool, offset: i32, limit: i32) -> anyhow::Result<Vec<Gist>> {
    let conn = pool.get().await?;
    let mut gists: Vec<Gist> = Vec::new();
    for row in conn
        .query(
            "SELECT g.id, g.user_id, g.description, g.filename, g.code, g.created, u.username, u.icon_url
             from gists as g
             join users as u on u.id = g.user_id
             order by g.id desc offset $1::int limit $2::int",
            &[&offset, &limit],
        )
        .await?
    {
        gists.push(gist_from_row(&row));
    }
    Ok(gists)
}

pub async fn user_gists(
    pool: &PgPool,
    username: &str,
    offset: i32,
    limit: i32,
) -> anyhow::Result<Vec<Gist>> {
    let conn = pool.get().await?;
    let mut gists: Vec<Gist> = Vec::new();
    for row in conn
        .query(
            "SELECT g.id, g.user_id, g.description, g.filename, g.code, g.created, u.username, u.icon_url
             from gists as g
             join users as u on u.id = g.user_id
             where u.username = $1
             order by g.id desc offset $2::int limit $3::int",
            &[&username, &offset, &limit],
        )
        .await?
    {
        gists.push(gist_from_row(&row));
    }
    Ok(gists)
}

pub async fn user_gists_count(pool: &PgPool, username: &str) -> anyhow::Result<i32> {
    let conn = pool.get().await?;
    let row = conn
        .query_one(
            "SELECT count(*)::int as count from gists as g
             join users as u on u.id = g.user_id
             where u.username = $1",
            &[&username],
        )
        .await?;
    Ok(row.get("count"))
}

pub async fn count(pool: &PgPool) -> anyhow::Result<i32> {
    let conn = pool.get().await?;
    let row = conn
        .query_one("SELECT count(*)::int as count from gists", &[])
        .await?;
    Ok(row.get("count"))
}

pub async fn get_by_id(pool: &PgPool, id: i32) -> anyhow::Result<Gist> {
    let conn = pool.get().await?;
    let row = conn
        .query_one(
            "SELECT g.id, g.user_id, g.description, g.filename, g.code, g.created, u.username, u.icon_url
             from gists as g join users as u on u.id=g.user_id
             where g.id = $1",
            &[&id],
        )
        .await?;
    Ok(gist_from_row(&row))
}

pub async fn update(
    pool: &PgPool,
    id: i32,
    description: &str,
    filename: &str,
    code: &str,
) -> anyhow::Result<()> {
    let conn = pool.get().await?;
    conn.execute(
        "UPDATE gists set description = $1, filename = $2, code = $3 WHERE id = $4",
        &[&description, &filename, &code, &id],
    )
    .await?;
    Ok(())
}

pub async fn delete_by_id(pool: &PgPool, id: i32) -> anyhow::Result<()> {
    let conn = pool.get().await?;
    conn.execute("DELETE FROM gists WHERE id = $1", &[&id])
        .await?;
    Ok(())
}

#[derive(Serialize, Debug, Default)]
pub struct Comment {
    pub id: i32,
    pub user_id: i32,
    pub gist_id: i32,
    pub body: String,
    pub created: NaiveDateTime,
    pub formated_created: String,
    pub user: models::user::User,
}

fn comment_from_row(row: &Row) -> Comment {
    let username: String = row.get("username");
    let created: NaiveDateTime = row.get("created");
    Comment {
        id: row.get("id"),
        user_id: row.get("user_id"),
        gist_id: row.get("gist_id"),
        body: row.get("body"),
        created,
        formated_created: util::jst_time_formatter(created),
        user: models::user::User {
            id: row.get("user_id"),
            username_hash: util::username_hash(&username),
            username,
            icon_url: row.get("icon_url"),
        },
    }
}

pub async fn add_comment(
    pool: &PgPool,
    user_id: i32,
    gist_id: i32,
    body: &str,
) -> anyhow::Result<()> {
    let conn = pool.get().await?;
    conn.execute(
        "INSERT INTO gist_comments (user_id, gist_id, body) VALUES ($1, $2, $3);",
        &[&user_id, &gist_id, &body],
    )
    .await?;
    Ok(())
}

pub async fn get_comments_by_gist_id(pool: &PgPool, id: i32) -> anyhow::Result<Vec<Comment>> {
    let conn = pool.get().await?;
    let mut comments: Vec<Comment> = Vec::new();
    for row in conn
        .query(
            "SELECT c.id, c.user_id, c.gist_id, c.body, c.created, u.username, u.icon_url
             from gist_comments as c join users as u on u.id = c.user_id
             where c.gist_id = $1 order by id asc",
            &[&id],
        )
        .await?
    {
        comments.push(comment_from_row(&row));
    }
    Ok(comments)
}

pub async fn get_comment_by_id(pool: &PgPool, id: i32) -> anyhow::Result<Comment> {
    let conn = pool.get().await?;
    let row = conn
        .query_one(
            "SELECT c.*, u.username, u.icon_url from gist_comments as c
             join users as u on u.id = c.user_id where c.id = $1",
            &[&id],
        )
        .await?;
    Ok(comment_from_row(&row))
}

pub async fn update_comment_by_id(pool: &PgPool, id: i32, body: &str) -> anyhow::Result<()> {
    let conn = pool.get().await?;
    conn.execute(
        "UPDATE gist_comments set body = $1 WHERE id = $2",
        &[&body, &id],
    )
    .await?;
    Ok(())
}

pub async fn delete_comment_by_id(pool: &PgPool, id: i32) -> anyhow::Result<()> {
    let conn = pool.get().await?;
    conn.execute("DELETE FROM gist_comments WHERE id = $1", &[&id])
        .await?;
    Ok(())
}
