use chrono::NaiveDateTime;
use serde::Serialize;
use tokio_postgres::Row;

use crate::db::PgPool;
use crate::models;
use crate::util;

#[derive(Serialize, Debug)]
pub struct Post {
    pub id: i32,
    pub kind: String,
    pub user_id: i32,
    pub title: String,
    pub body: String,
    pub created: NaiveDateTime,
    pub formated_created: String,
    pub user: models::user::User,
    pub tags: Vec<models::tag::Tag>,
    pub shared: bool,
    pub status: String,
}

/// Build `Post` values (including their tags) from rows that select the
/// standard post columns joined with users.
pub async fn posts_from_rows(pool: &PgPool, rows: Vec<Row>) -> anyhow::Result<Vec<Post>> {
    let mut posts: Vec<Post> = Vec::new();
    for row in rows {
        let id: i32 = row.get("id");
        let tags = models::tag::get_tags_by_post_id(pool, id).await?;
        posts.push(post_from_row(&row, tags));
    }
    Ok(posts)
}

fn post_from_row(row: &Row, tags: Vec<models::tag::Tag>) -> Post {
    let username: String = row.get("username");
    let created: NaiveDateTime = row.get("created");
    Post {
        id: row.get("id"),
        kind: row.get("kind"),
        user_id: row.get("user_id"),
        title: row.get("title"),
        body: row.get("body"),
        created,
        formated_created: util::jst_time_formatter(created),
        shared: row.get("shared"),
        status: row.get("status"),
        user: models::user::User {
            id: row.get("user_id"),
            username_hash: util::username_hash(&username),
            username,
            icon_url: row.get("icon_url"),
        },
        tags,
    }
}

pub async fn create(
    pool: &PgPool,
    kind: &str,
    user_id: i32,
    action: &str,
    title: &str,
    body: &str,
    tags: &str,
) -> anyhow::Result<i32> {
    let conn = pool.get().await?;
    let row = conn
        .query_one(
            "INSERT INTO posts (kind, user_id, title, body, status)
             VALUES ($1, $2, $3, $4, $5) returning id;",
            &[&kind, &user_id, &title, &body, &action],
        )
        .await?;
    let post_id: i32 = row.get("id");
    drop(conn);
    for tag in tags.split(',') {
        let tag = tag.trim();
        if !tag.is_empty() {
            let tag_id = models::tag::select_or_create_tag_id(pool, tag).await?;
            let conn = pool.get().await?;
            conn.execute(
                "INSERT INTO taggings (tag_id, post_id) VALUES ($1, $2);",
                &[&tag_id, &post_id],
            )
            .await?;
        }
    }
    Ok(post_id)
}

pub async fn list(pool: &PgPool, kind: &str, offset: i32, limit: i32) -> anyhow::Result<Vec<Post>> {
    let conn = pool.get().await?;
    let rows = conn
        .query(
            "SELECT p.id, p.kind, p.user_id, p.title, p.body, p.created, p.shared, p.status, u.username, u.icon_url
             from posts as p
             join users as u on u.id = p.user_id
             where p.status = 'publish' and p.kind = $1
             order by p.id desc offset $2::int limit $3::int",
            &[&kind, &offset, &limit],
        )
        .await?;
    drop(conn);
    posts_from_rows(pool, rows).await
}

pub async fn count(pool: &PgPool, kind: &str) -> anyhow::Result<i32> {
    let conn = pool.get().await?;
    let row = conn
        .query_one(
            "SELECT count(*)::int as count from posts where status = 'publish' and kind = $1",
            &[&kind],
        )
        .await?;
    Ok(row.get("count"))
}

pub async fn update(
    pool: &PgPool,
    id: i32,
    title: &str,
    body: &str,
    tags: &str,
    action: &str,
) -> anyhow::Result<()> {
    let conn = pool.get().await?;
    conn.execute(
        "UPDATE posts set title = $1, body = $2, status = $3 WHERE id = $4",
        &[&title, &body, &action, &id],
    )
    .await?;
    drop(conn);

    let mut old_tag_ids: Vec<i32> = models::tag::get_tags_by_post_id(pool, id)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|t| t.id)
        .collect();

    let mut new_tag_ids: Vec<i32> = Vec::new();
    for tag in tags.split(',') {
        let tag = tag.trim();
        if tag.is_empty() {
            continue;
        }
        if let Ok(tag_id) = models::tag::select_or_create_tag_id(pool, tag).await {
            if let Some(index) = old_tag_ids.iter().position(|r| *r == tag_id) {
                old_tag_ids.remove(index);
            } else {
                new_tag_ids.push(tag_id);
            }
        }
    }
    new_tag_ids.dedup();

    let conn = pool.get().await?;
    for tag_id in &new_tag_ids {
        conn.execute(
            "INSERT INTO taggings (tag_id, post_id)
             SELECT $1, $2
             WHERE NOT EXISTS (SELECT * FROM taggings WHERE tag_id = $1 AND post_id = $2)",
            &[tag_id, &id],
        )
        .await?;
    }
    for tag_id in &old_tag_ids {
        conn.execute(
            "DELETE FROM taggings where tag_id = $1 and post_id = $2;",
            &[tag_id, &id],
        )
        .await?;
    }
    Ok(())
}

pub async fn get_by_id(pool: &PgPool, id: i32) -> anyhow::Result<Post> {
    let conn = pool.get().await?;
    let row = conn
        .query_one(
            "SELECT p.id, p.kind, p.user_id, p.title, p.body, p.created, p.shared, p.status, u.username, u.icon_url
             from posts as p join users as u on u.id=p.user_id where p.id = $1",
            &[&id],
        )
        .await?;
    drop(conn);
    let tags = models::tag::get_tags_by_post_id(pool, id).await?;
    Ok(post_from_row(&row, tags))
}

pub async fn delete_by_id(pool: &PgPool, id: i32) -> anyhow::Result<()> {
    let conn = pool.get().await?;
    conn.execute("DELETE FROM posts WHERE id = $1", &[&id])
        .await?;
    Ok(())
}

#[derive(Serialize, Debug, Default)]
pub struct Comment {
    pub id: i32,
    pub user_id: i32,
    pub post_id: i32,
    pub body: String,
    pub user: models::user::User,
}

pub async fn add_comment(
    pool: &PgPool,
    user_id: i32,
    post_id: i32,
    body: &str,
) -> anyhow::Result<i32> {
    let conn = pool.get().await?;
    conn.query_one(
        "INSERT INTO post_comments (user_id, post_id, body)
         VALUES ($1, $2, $3) returning id;",
        &[&user_id, &post_id, &body],
    )
    .await?;
    let post = conn
        .query_one("SELECT * from posts where id = $1", &[&post_id])
        .await?;
    let post_user_id: i32 = post.get("user_id");
    drop(conn);
    models::notification::create(
        pool,
        &format!("/post/show/{}", post_id),
        user_id,
        post_user_id,
        body,
        "comment",
    )
    .await
}

fn comment_from_row(row: &Row) -> Comment {
    let username: String = row.get("username");
    Comment {
        id: row.get("id"),
        user_id: row.get("user_id"),
        post_id: row.get("post_id"),
        body: row.get("body"),
        user: models::user::User {
            id: row.get("user_id"),
            username_hash: util::username_hash(&username),
            username,
            icon_url: row.get("icon_url"),
        },
    }
}

pub async fn get_comments_by_post_id(pool: &PgPool, id: i32) -> anyhow::Result<Vec<Comment>> {
    let conn = pool.get().await?;
    let mut comments: Vec<Comment> = Vec::new();
    for row in conn
        .query(
            "SELECT c.id, c.user_id, c.post_id, c.body, u.username, u.icon_url
             from post_comments as c join users as u on u.id = c.user_id
             where c.post_id = $1 order by id asc",
            &[&id],
        )
        .await?
    {
        comments.push(comment_from_row(&row));
    }
    Ok(comments)
}

#[derive(Serialize, Debug)]
pub struct Feed {
    pub id: i32,
    pub post: String,
    pub comment: String,
    pub gist: String,
    pub kind: String,
    pub user_id: i32,
    pub title: String,
    pub body: String,
    pub created: NaiveDateTime,
    pub formated_created: String,
    pub user: models::user::User,
    pub tags: Vec<models::tag::Tag>,
}

pub async fn get_feeds(pool: &PgPool, offset: i32, limit: i32) -> anyhow::Result<Vec<Feed>> {
    let conn = pool.get().await?;
    let rows = conn
        .query(
            "(select '' as comment, '' as gist, 'post' as post, p.id, p.kind, p.user_id, p.title, '' as body, u.username, u.icon_url, p.created from posts as p join users as u on u.id=p.user_id where p.status = 'publish')
             union
             (select 'comment' as comment, '' as gist, '' as post, c.post_id, p.kind, c.user_id, p.title as title, c.body, u.username, u.icon_url, c.created from post_comments as c join users as u on u.id=c.user_id join posts as p on c.post_id=p.id)
             union
             (select '' as comment, 'gist' as gist, '' as post, g.id, '' as kind, g.user_id, g.description as title, '' as body, u.username, u.icon_url, g.created from gists as g join users as u on u.id=g.user_id)
             order by created desc offset $1::int limit $2::int",
            &[&offset, &limit],
        )
        .await?;
    drop(conn);

    let mut feeds: Vec<Feed> = Vec::new();
    for row in rows {
        let id: i32 = row.get("id");
        let tags = models::tag::get_tags_by_post_id(pool, id).await?;
        let body: String = row.get("body");
        let body: String = body.chars().take(50).collect();
        let username: String = row.get("username");
        let created: NaiveDateTime = row.get("created");
        feeds.push(Feed {
            id,
            post: row.get("post"),
            comment: row.get("comment"),
            gist: row.get("gist"),
            kind: row.get("kind"),
            user_id: row.get("user_id"),
            title: row.get("title"),
            body,
            created,
            formated_created: util::jst_time_formatter(created),
            user: models::user::User {
                id: row.get("user_id"),
                username_hash: util::username_hash(&username),
                username,
                icon_url: row.get("icon_url"),
            },
            tags,
        });
    }
    Ok(feeds)
}

pub async fn get_feed_count(pool: &PgPool) -> anyhow::Result<i32> {
    let conn = pool.get().await?;
    let row = conn
        .query_one(
            "select sum(count)::int as count from
             (select count(*) from posts where status = 'publish'
              union all
              select count(*) as b from post_comments) as t;",
            &[],
        )
        .await?;
    Ok(row.get("count"))
}

pub async fn stock_post(pool: &PgPool, user_id: i32, post_id: i32) -> anyhow::Result<()> {
    let conn = pool.get().await?;
    conn.execute(
        "INSERT INTO stocks (user_id, post_id) VALUES ($1, $2);",
        &[&user_id, &post_id],
    )
    .await?;
    Ok(())
}

pub async fn stocked_list(
    pool: &PgPool,
    user_id: i32,
    offset: i32,
    limit: i32,
) -> anyhow::Result<Vec<Post>> {
    let conn = pool.get().await?;
    let rows = conn
        .query(
            "SELECT p.id, p.kind, p.user_id, p.title, p.body, p.created, p.shared, p.status, u.username, u.icon_url
             from posts as p
             join stocks as s on s.post_id = p.id
             join users as u on u.id = p.user_id
             where s.user_id = $1
             order by s.id desc offset $2::int limit $3::int",
            &[&user_id, &offset, &limit],
        )
        .await?;
    drop(conn);
    posts_from_rows(pool, rows).await
}

pub async fn stocked_count(pool: &PgPool, user_id: i32) -> anyhow::Result<i32> {
    let conn = pool.get().await?;
    let row = conn
        .query_one(
            "SELECT count(*)::int as count from stocks where user_id = $1",
            &[&user_id],
        )
        .await?;
    Ok(row.get("count"))
}

pub async fn is_stocked(pool: &PgPool, user_id: i32, post_id: i32) -> anyhow::Result<bool> {
    let conn = pool.get().await?;
    let row = conn
        .query_one(
            "SELECT count(*)::int as count from stocks where user_id = $1 and post_id = $2",
            &[&user_id, &post_id],
        )
        .await?;
    let count: i32 = row.get("count");
    Ok(count > 0)
}

pub async fn stock_remove(pool: &PgPool, user_id: i32, post_id: i32) -> anyhow::Result<()> {
    let conn = pool.get().await?;
    conn.execute(
        "delete from stocks where user_id = $1 and post_id = $2",
        &[&user_id, &post_id],
    )
    .await?;
    Ok(())
}

pub async fn draft_list(pool: &PgPool, user_id: i32) -> anyhow::Result<Vec<Post>> {
    let conn = pool.get().await?;
    let rows = conn
        .query(
            "SELECT p.id, p.kind, p.user_id, p.title, p.body, p.created, p.shared, p.status, u.username, u.icon_url
             from posts as p
             join users as u on u.id = p.user_id
             where p.status = 'draft' and p.user_id = $1
             order by p.id desc",
            &[&user_id],
        )
        .await?;
    drop(conn);
    posts_from_rows(pool, rows).await
}

pub async fn get_comment_by_id(pool: &PgPool, id: i32) -> anyhow::Result<Comment> {
    let conn = pool.get().await?;
    let row = conn
        .query_one(
            "SELECT p.*, u.username, u.icon_url from post_comments as p
             join users as u on u.id = p.user_id where p.id = $1",
            &[&id],
        )
        .await?;
    Ok(comment_from_row(&row))
}

pub async fn update_comment_by_id(pool: &PgPool, id: i32, body: &str) -> anyhow::Result<()> {
    let conn = pool.get().await?;
    conn.execute(
        "UPDATE post_comments set body = $1 WHERE id = $2",
        &[&body, &id],
    )
    .await?;
    Ok(())
}

pub async fn delete_comment_by_id(pool: &PgPool, id: i32) -> anyhow::Result<()> {
    let conn = pool.get().await?;
    conn.execute("DELETE FROM post_comments WHERE id = $1", &[&id])
        .await?;
    Ok(())
}

pub async fn user_posts(
    pool: &PgPool,
    username: &str,
    offset: i32,
    limit: i32,
    kind: &str,
) -> anyhow::Result<Vec<Post>> {
    let conn = pool.get().await?;
    let rows = conn
        .query(
            "SELECT p.id, p.kind, p.user_id, p.title, p.body, p.created, p.shared, p.status, u.username, u.icon_url
             from posts as p
             join users as u on u.id = p.user_id
             where p.status = 'publish' and u.username = $1 and p.kind = $2
             order by p.id desc offset $3::int limit $4::int",
            &[&username, &kind, &offset, &limit],
        )
        .await?;
    drop(conn);
    posts_from_rows(pool, rows).await
}

pub async fn user_posts_count(pool: &PgPool, username: &str, kind: &str) -> anyhow::Result<i32> {
    let conn = pool.get().await?;
    let row = conn
        .query_one(
            "SELECT count(*)::int as count from posts as p
             join users as u on u.id = p.user_id
             where p.status = 'publish' and u.username = $1 and p.kind = $2",
            &[&username, &kind],
        )
        .await?;
    Ok(row.get("count"))
}

pub async fn share_post(pool: &PgPool, post_id: i32) -> anyhow::Result<()> {
    let conn = pool.get().await?;
    conn.execute("update posts set shared = true where id = $1", &[&post_id])
        .await?;
    Ok(())
}

pub async fn pin_post(pool: &PgPool, user_id: i32, post_id: i32) -> anyhow::Result<()> {
    let conn = pool.get().await?;
    conn.execute(
        "INSERT INTO pinneds (user_id, post_id) VALUES ($1, $2);",
        &[&user_id, &post_id],
    )
    .await?;
    Ok(())
}

pub async fn unpin_post(pool: &PgPool, post_id: i32) -> anyhow::Result<()> {
    let conn = pool.get().await?;
    conn.execute(
        "update pinneds set deleted = true where post_id = $1",
        &[&post_id],
    )
    .await?;
    Ok(())
}

pub async fn pinned_list(pool: &PgPool, offset: i32, limit: i32) -> anyhow::Result<Vec<Post>> {
    let conn = pool.get().await?;
    let rows = conn
        .query(
            "SELECT p.id, p.kind, p.user_id, p.title, p.body, p.created, p.shared, p.status, u.username, u.icon_url
             from posts as p
             join pinneds as s on s.post_id = p.id
             join users as u on u.id = p.user_id
             where s.deleted = false
             order by s.id desc offset $1::int limit $2::int",
            &[&offset, &limit],
        )
        .await?;
    drop(conn);
    posts_from_rows(pool, rows).await
}

pub async fn is_pinned(pool: &PgPool, post_id: i32) -> anyhow::Result<bool> {
    let conn = pool.get().await?;
    let row = conn
        .query_one(
            "SELECT count(*)::int as count from pinneds where deleted = false and post_id = $1",
            &[&post_id],
        )
        .await?;
    let count: i32 = row.get("count");
    Ok(count > 0)
}

pub async fn pinned_count(pool: &PgPool) -> anyhow::Result<i32> {
    let conn = pool.get().await?;
    let row = conn
        .query_one(
            "SELECT count(*)::int as count from pinneds where deleted = false",
            &[],
        )
        .await?;
    Ok(row.get("count"))
}
