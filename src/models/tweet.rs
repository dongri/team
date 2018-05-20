use postgres::error::Error;
use db;
use models;
use helper;
use chrono::{NaiveDateTime};

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

pub fn create(conn: &db::PostgresConnection, user_id: &i32, body: &String) -> Result<(i32), Error> {
    let mut post_id = 0;
    for row in &conn.query("
        INSERT INTO tweets (user_id, body)
        VALUES ($1, $2) returning id;",
        &[&user_id, &body]).unwrap() {
        post_id = row.get("id");
    }
    Ok(post_id)
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

pub fn add_comment(conn: &db::PostgresConnection, user_id: &i32, tweet_id: &i32, body: &String) -> Result<(i32), Error> {
    &conn.query("
        INSERT INTO tweet_comments (user_id, tweet_id, body) 
        VALUES ($1, $2, $3) returning id;",
        &[&user_id, &tweet_id, &body]).unwrap();
    let tweets = &conn.query("SELECT * from tweets where id = $1", &[&tweet_id]).unwrap();
    let tweet = tweets.get(0);
    let tweet_user_id: i32 = tweet.get("user_id");
    models::notification::create(conn, &format!("/tweet/show/{}", tweet_id).to_string(), user_id, &tweet_user_id, body)
}

pub fn list(conn: &db::PostgresConnection, offset: &i32, limit: &i32) -> Result<Vec<Tweet>, Error> {
    let mut tweets: Vec<Tweet> = Vec::new();
    for row in &conn.query("
        select t.id, t.user_id, t.body, t.created, u.username, u.icon_url,
        (select count(*)::int from tweet_comments as c where c.tweet_id = t.id) as comment_count
        from tweets as t
        join users as u on u.id = t.user_id
        order by t.id desc offset $1::int limit $2::int", &[&offset, &limit]).unwrap() {
        let mut tweet = Tweet {
            id: row.get("id"),
            user_id: row.get("user_id"),
            body: row.get("body"),
            created: row.get("created"),
            formated_created: "".to_string(),
            comment_count: row.get("comment_count"),
            user: models::user::User{
                id: row.get("user_id"),
                username: row.get("username"),
                icon_url: row.get("icon_url"),
                username_hash: helper::username_hash(row.get("username")),
            },
        };
        tweet.formated_created = helper::jst_time_formatter(tweet.created);
        tweets.push(tweet);
    }
    Ok(tweets)
}

pub fn count(conn: &db::PostgresConnection) -> Result<i32, Error> {
    let rows = &conn.query("SELECT count(*)::int as count from tweets", &[]).unwrap();
    let row = rows.get(0);
    let count = row.get("count");
    Ok(count)
}

pub fn get_by_id(conn: &db::PostgresConnection, id: &i32) -> Result<Tweet, Error> {
    let rows = &conn.query("
        select t.id, t.user_id, t.body, t.created, u.username, u.icon_url,
        (select count(*)::int from tweet_comments as c where c.tweet_id = t.id) as comment_count
        from tweets as t join users as u on u.id=t.user_id 
        where t.id = $1", &[&id]).unwrap();
    let row = rows.get(0);
    let mut tweet = Tweet {
        id: row.get("id"),
        user_id: row.get("user_id"),
        body: row.get("body"),
        created: row.get("created"),
        formated_created: "".to_string(),
        comment_count: row.get("comment_count"),
        user: models::user::User{
            id: row.get("user_id"),
            username: row.get("username"),
            icon_url: row.get("icon_url"),
            username_hash: helper::username_hash(row.get("username")),
        },
    };
    tweet.formated_created = helper::jst_time_formatter(tweet.created);
    Ok(tweet)
}

pub fn get_comments_by_tweet_id(conn: &db::PostgresConnection, id: &i32) -> Result<Vec<Comment>, Error> {
    let mut comments: Vec<Comment> = Vec::new();
    for row in &conn.query("SELECT c.id, c.user_id, c.tweet_id, c.body, c.created,u.username, u.icon_url from tweet_comments as c join users as u on u.id = c.user_id where c.tweet_id = $1 order by c.id asc", &[&id]).unwrap() {
        comments.push(Comment {
            id: row.get("id"),
            user_id: row.get("user_id"),
            tweet_id: row.get("tweet_id"),
            body: row.get("body"),
            created: row.get("created"),
            formated_created: "".to_string(),
            user: models::user::User{
                id: row.get("user_id"),
                username: row.get("username"),
                icon_url: row.get("icon_url"),
                username_hash: helper::username_hash(row.get("username")),
            }
        });
    }
    Ok(comments)
}
