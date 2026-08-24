use chrono::NaiveDateTime;
use serde::Serialize;
use tokio_postgres::Row;

use crate::db::PgPool;
use crate::models;
use crate::util;

#[derive(Serialize, Debug)]
pub struct Tweet {
    pub id: i32,
    pub user_id: i32,
    pub body: String,
    pub created: NaiveDateTime,
    pub formated_created: String,
    pub comment_count: i32,
    pub user: models::user::User,
}

fn tweet_from_row(row: &Row) -> Tweet {
    let username: String = row.get("username");
    let created: NaiveDateTime = row.get("created");
    Tweet {
        id: row.get("id"),
        user_id: row.get("user_id"),
        body: row.get("body"),
        created,
        formated_created: util::jst_time_formatter(created),
        comment_count: row.get("comment_count"),
        user: models::user::User {
            id: row.get("user_id"),
            username_hash: util::username_hash(&username),
            username,
            icon_url: row.get("icon_url"),
        },
    }
}

pub async fn create(pool: &PgPool, user_id: i32, body: &str) -> anyhow::Result<i32> {
    let conn = pool.get().await?;
    let row = conn
        .query_one(
            "INSERT INTO tweets (user_id, body) VALUES ($1, $2) returning id;",
            &[&user_id, &body],
        )
        .await?;
    Ok(row.get("id"))
}

#[derive(Serialize, Debug)]
pub struct Comment {
    pub id: i32,
    pub user_id: i32,
    pub tweet_id: i32,
    pub body: String,
    pub created: NaiveDateTime,
    pub formated_created: String,
    pub user: models::user::User,
}

pub async fn add_comment(
    pool: &PgPool,
    user_id: i32,
    tweet_id: i32,
    body: &str,
) -> anyhow::Result<i32> {
    let conn = pool.get().await?;
    conn.query_one(
        "INSERT INTO tweet_comments (user_id, tweet_id, body)
         VALUES ($1, $2, $3) returning id;",
        &[&user_id, &tweet_id, &body],
    )
    .await?;
    let tweet = conn
        .query_one("SELECT * from tweets where id = $1", &[&tweet_id])
        .await?;
    let tweet_user_id: i32 = tweet.get("user_id");
    drop(conn);
    models::notification::create(
        pool,
        &format!("/tweet/show/{}", tweet_id),
        user_id,
        tweet_user_id,
        body,
    )
    .await
}

pub async fn list(pool: &PgPool, offset: i32, limit: i32) -> anyhow::Result<Vec<Tweet>> {
    let conn = pool.get().await?;
    let mut tweets: Vec<Tweet> = Vec::new();
    for row in conn
        .query(
            "select t.id, t.user_id, t.body, t.created, u.username, u.icon_url,
             (select count(*)::int from tweet_comments as c where c.tweet_id = t.id) as comment_count
             from tweets as t
             join users as u on u.id = t.user_id
             order by t.id desc offset $1::int limit $2::int",
            &[&offset, &limit],
        )
        .await?
    {
        tweets.push(tweet_from_row(&row));
    }
    Ok(tweets)
}

pub async fn count(pool: &PgPool) -> anyhow::Result<i32> {
    let conn = pool.get().await?;
    let row = conn
        .query_one("SELECT count(*)::int as count from tweets", &[])
        .await?;
    Ok(row.get("count"))
}

pub async fn get_by_id(pool: &PgPool, id: i32) -> anyhow::Result<Tweet> {
    let conn = pool.get().await?;
    let row = conn
        .query_one(
            "select t.id, t.user_id, t.body, t.created, u.username, u.icon_url,
             (select count(*)::int from tweet_comments as c where c.tweet_id = t.id) as comment_count
             from tweets as t join users as u on u.id=t.user_id
             where t.id = $1",
            &[&id],
        )
        .await?;
    Ok(tweet_from_row(&row))
}

pub async fn get_comments_by_tweet_id(pool: &PgPool, id: i32) -> anyhow::Result<Vec<Comment>> {
    let conn = pool.get().await?;
    let mut comments: Vec<Comment> = Vec::new();
    for row in conn
        .query(
            "SELECT c.id, c.user_id, c.tweet_id, c.body, c.created, u.username, u.icon_url
             from tweet_comments as c join users as u on u.id = c.user_id
             where c.tweet_id = $1 order by c.id asc",
            &[&id],
        )
        .await?
    {
        let username: String = row.get("username");
        let created: NaiveDateTime = row.get("created");
        comments.push(Comment {
            id: row.get("id"),
            user_id: row.get("user_id"),
            tweet_id: row.get("tweet_id"),
            body: row.get("body"),
            created,
            formated_created: util::jst_time_formatter(created),
            user: models::user::User {
                id: row.get("user_id"),
                username_hash: util::username_hash(&username),
                username,
                icon_url: row.get("icon_url"),
            },
        });
    }
    Ok(comments)
}
