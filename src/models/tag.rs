use serde::Serialize;

use crate::db::PgPool;
use crate::models;

#[derive(Serialize, Debug, Default)]
pub struct Tag {
    pub id: i32,
    pub name: String,
}

pub async fn select_or_create_tag_id(pool: &PgPool, tag_name: &str) -> anyhow::Result<i32> {
    let conn = pool.get().await?;
    let rows = conn
        .query("SELECT id from tags where name = $1", &[&tag_name])
        .await?;
    if let Some(row) = rows.first() {
        return Ok(row.get("id"));
    }
    let row = conn
        .query_one(
            "INSERT INTO tags (name) VALUES ($1) returning id;",
            &[&tag_name],
        )
        .await?;
    Ok(row.get("id"))
}

pub async fn get_tags_by_post_id(pool: &PgPool, post_id: i32) -> anyhow::Result<Vec<Tag>> {
    let conn = pool.get().await?;
    let mut tags: Vec<Tag> = Vec::new();
    for row in conn
        .query(
            "select t2.id, t2.name from taggings as t1
             join tags as t2 on t1.tag_id = t2.id
             where t1.post_id = $1 order by t2.id desc",
            &[&post_id],
        )
        .await?
    {
        tags.push(Tag {
            id: row.get("id"),
            name: row.get("name"),
        });
    }
    Ok(tags)
}

pub async fn tag_search(
    pool: &PgPool,
    tag_name: &str,
    offset: i32,
    limit: i32,
) -> anyhow::Result<Vec<models::post::Post>> {
    let conn = pool.get().await?;
    let rows = conn
        .query(
            "SELECT p.id, p.kind, p.user_id, p.title, p.body, p.created, p.shared, p.status, u.username, u.icon_url
             from posts as p
             join users as u on u.id = p.user_id
             join taggings as t on p.id = t.post_id
             join tags as tg on t.tag_id = tg.id
             where tg.name = $1
             order by p.id desc offset $2::int limit $3::int",
            &[&tag_name, &offset, &limit],
        )
        .await?;
    drop(conn);
    models::post::posts_from_rows(pool, rows).await
}

pub async fn tag_count(pool: &PgPool, tag_name: &str) -> anyhow::Result<i32> {
    let conn = pool.get().await?;
    let row = conn
        .query_one(
            "SELECT count(p.*)::int as count from posts as p
             join taggings as t on p.id = t.post_id
             join tags tg on t.tag_id = tg.id
             where tg.name = $1",
            &[&tag_name],
        )
        .await?;
    Ok(row.get("count"))
}

pub async fn tag_list(pool: &PgPool) -> anyhow::Result<Vec<Tag>> {
    let conn = pool.get().await?;
    let mut tags: Vec<Tag> = Vec::new();
    for row in conn
        .query("select * from tags order by id desc", &[])
        .await?
    {
        tags.push(Tag {
            id: row.get("id"),
            name: row.get("name"),
        });
    }
    Ok(tags)
}
