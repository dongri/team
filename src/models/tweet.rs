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
    pub like_count: i32,
    pub liked: bool,
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
        like_count: row.get("like_count"),
        liked: row.get("liked"),
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
        "comment",
    )
    .await
}

fn comment_from_row(row: &Row) -> Comment {
    let username: String = row.get("username");
    let created: NaiveDateTime = row.get("created");
    Comment {
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
    }
}

pub async fn get_comment_by_id(pool: &PgPool, id: i32) -> anyhow::Result<Comment> {
    let conn = pool.get().await?;
    let row = conn
        .query_one(
            "SELECT c.*, u.username, u.icon_url from tweet_comments as c
             join users as u on u.id = c.user_id where c.id = $1",
            &[&id],
        )
        .await?;
    Ok(comment_from_row(&row))
}

pub async fn update_comment_by_id(pool: &PgPool, id: i32, body: &str) -> anyhow::Result<()> {
    let conn = pool.get().await?;
    conn.execute(
        "UPDATE tweet_comments set body = $1 WHERE id = $2",
        &[&body, &id],
    )
    .await?;
    Ok(())
}

pub async fn delete_comment_by_id(pool: &PgPool, id: i32) -> anyhow::Result<()> {
    let conn = pool.get().await?;
    conn.execute("DELETE FROM tweet_comments WHERE id = $1", &[&id])
        .await?;
    Ok(())
}

pub async fn list(
    pool: &PgPool,
    login_user_id: i32,
    offset: i32,
    limit: i32,
) -> anyhow::Result<Vec<Tweet>> {
    let conn = pool.get().await?;
    let mut tweets: Vec<Tweet> = Vec::new();
    for row in conn
        .query(
            "select t.id, t.user_id, t.body, t.created, u.username, u.icon_url,
             (select count(*)::int from tweet_comments as c where c.tweet_id = t.id) as comment_count,
             (select count(*)::int from tweet_likes as l where l.tweet_id = t.id) as like_count,
             exists(select 1 from tweet_likes as l where l.tweet_id = t.id and l.user_id = $3) as liked
             from tweets as t
             join users as u on u.id = t.user_id
             order by t.id desc offset $1::int limit $2::int",
            &[&offset, &limit, &login_user_id],
        )
        .await?
    {
        tweets.push(tweet_from_row(&row));
    }
    Ok(tweets)
}

pub async fn user_tweets(
    pool: &PgPool,
    login_user_id: i32,
    username: &str,
    offset: i32,
    limit: i32,
) -> anyhow::Result<Vec<Tweet>> {
    let conn = pool.get().await?;
    let mut tweets: Vec<Tweet> = Vec::new();
    for row in conn
        .query(
            "select t.id, t.user_id, t.body, t.created, u.username, u.icon_url,
             (select count(*)::int from tweet_comments as c where c.tweet_id = t.id) as comment_count,
             (select count(*)::int from tweet_likes as l where l.tweet_id = t.id) as like_count,
             exists(select 1 from tweet_likes as l where l.tweet_id = t.id and l.user_id = $4) as liked
             from tweets as t
             join users as u on u.id = t.user_id
             where u.username = $1
             order by t.id desc offset $2::int limit $3::int",
            &[&username, &offset, &limit, &login_user_id],
        )
        .await?
    {
        tweets.push(tweet_from_row(&row));
    }
    Ok(tweets)
}

pub async fn user_tweets_count(pool: &PgPool, username: &str) -> anyhow::Result<i32> {
    let conn = pool.get().await?;
    let row = conn
        .query_one(
            "SELECT count(*)::int as count from tweets as t
             join users as u on u.id = t.user_id
             where u.username = $1",
            &[&username],
        )
        .await?;
    Ok(row.get("count"))
}

pub async fn count(pool: &PgPool) -> anyhow::Result<i32> {
    let conn = pool.get().await?;
    let row = conn
        .query_one("SELECT count(*)::int as count from tweets", &[])
        .await?;
    Ok(row.get("count"))
}

pub async fn get_by_id(pool: &PgPool, login_user_id: i32, id: i32) -> anyhow::Result<Tweet> {
    let conn = pool.get().await?;
    let row = conn
        .query_one(
            "select t.id, t.user_id, t.body, t.created, u.username, u.icon_url,
             (select count(*)::int from tweet_comments as c where c.tweet_id = t.id) as comment_count,
             (select count(*)::int from tweet_likes as l where l.tweet_id = t.id) as like_count,
             exists(select 1 from tweet_likes as l where l.tweet_id = t.id and l.user_id = $2) as liked
             from tweets as t join users as u on u.id=t.user_id
             where t.id = $1",
            &[&id, &login_user_id],
        )
        .await?;
    Ok(tweet_from_row(&row))
}

/// Toggle the login user's like on a tweet. Returns the new liked state.
/// Liking notifies the tweet's owner; unliking removes that notification.
pub async fn toggle_like(pool: &PgPool, user_id: i32, tweet_id: i32) -> anyhow::Result<bool> {
    let path = format!("/tweet/show/{}", tweet_id);
    let conn = pool.get().await?;
    let deleted = conn
        .execute(
            "delete from tweet_likes where user_id = $1 and tweet_id = $2",
            &[&user_id, &tweet_id],
        )
        .await?;
    if deleted > 0 {
        drop(conn);
        models::notification::delete_likes(pool, &path, user_id).await?;
        return Ok(false);
    }
    conn.execute(
        "INSERT INTO tweet_likes (user_id, tweet_id) VALUES ($1, $2)
         ON CONFLICT (user_id, tweet_id) DO NOTHING;",
        &[&user_id, &tweet_id],
    )
    .await?;
    let tweet = conn
        .query_one(
            "SELECT user_id, body from tweets where id = $1",
            &[&tweet_id],
        )
        .await?;
    let tweet_user_id: i32 = tweet.get("user_id");
    let tweet_body: String = tweet.get("body");
    drop(conn);
    if tweet_user_id != user_id {
        let excerpt: String = tweet_body.chars().take(40).collect();
        models::notification::create(pool, &path, user_id, tweet_user_id, &excerpt, "like").await?;
    }
    Ok(true)
}

pub async fn like_count(pool: &PgPool, tweet_id: i32) -> anyhow::Result<i32> {
    let conn = pool.get().await?;
    let row = conn
        .query_one(
            "SELECT count(*)::int as count from tweet_likes where tweet_id = $1",
            &[&tweet_id],
        )
        .await?;
    Ok(row.get("count"))
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
        comments.push(comment_from_row(&row));
    }
    Ok(comments)
}
