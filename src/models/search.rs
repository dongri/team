use chrono::NaiveDateTime;
use serde::Serialize;

use crate::db::PgPool;
use crate::models;
use crate::util;

#[derive(Serialize, Debug)]
pub struct SearchResult {
    // 'post' | 'nippo' | 'gist' | 'tweet' (a comment carries its parent's category)
    pub category: String,
    pub path: String,
    pub title: String,
    pub body: String,
    pub is_comment: bool,
    pub is_tweet: bool,
    pub created: NaiveDateTime,
    pub formated_created: String,
    pub user: models::user::User,
}

// Everything searchable, one row per match: published posts/nippos
// (title/body), gists (description/filename/code), tweets (body), and the
// comments on each of them. Column names/types come from the first branch.
const SEARCH_UNION: &str = "
  (select p.kind as category, '/' || p.kind || '/show/' || p.id as path,
          p.title as title, p.body as body, false as is_comment,
          p.created as created, p.user_id as user_id, u.username, u.icon_url
   from posts as p join users as u on u.id = p.user_id
   where p.status = 'publish' and (p.title ilike '%' || $1 || '%' or p.body ilike '%' || $1 || '%'))
  union all
  (select 'gist', '/gist/show/' || g.id,
          coalesce(g.description, g.filename, ''), g.code, false,
          g.created, g.user_id, u.username, u.icon_url
   from gists as g join users as u on u.id = g.user_id
   where g.description ilike '%' || $1 || '%' or g.filename ilike '%' || $1 || '%' or g.code ilike '%' || $1 || '%')
  union all
  (select 'tweet', '/tweet/show/' || t.id, '', t.body, false,
          t.created, t.user_id, u.username, u.icon_url
   from tweets as t join users as u on u.id = t.user_id
   where t.body ilike '%' || $1 || '%')
  union all
  (select p.kind, '/' || p.kind || '/show/' || p.id, p.title, c.body, true,
          c.created, c.user_id, u.username, u.icon_url
   from post_comments as c
   join posts as p on p.id = c.post_id
   join users as u on u.id = c.user_id
   where p.status = 'publish' and c.body ilike '%' || $1 || '%')
  union all
  (select 'gist', '/gist/show/' || g.id, coalesce(g.description, g.filename, ''), c.body, true,
          c.created, c.user_id, u.username, u.icon_url
   from gist_comments as c
   join gists as g on g.id = c.gist_id
   join users as u on u.id = c.user_id
   where c.body ilike '%' || $1 || '%')
  union all
  (select 'tweet', '/tweet/show/' || t.id, '', c.body, true,
          c.created, c.user_id, u.username, u.icon_url
   from tweet_comments as c
   join tweets as t on t.id = c.tweet_id
   join users as u on u.id = c.user_id
   where c.body ilike '%' || $1 || '%')
";

// $2 is the tab: 'all' shows everything, 'comment' every comment, and a
// category tab shows that category's own items without its comments.
const SEARCH_FILTER: &str = "
  ($2 = 'all')
  or ($2 = 'comment' and s.is_comment)
  or ($2 = s.category and not s.is_comment)
";

pub async fn search(
    pool: &PgPool,
    keyword: &str,
    kind: &str,
    offset: i32,
    limit: i32,
) -> anyhow::Result<Vec<SearchResult>> {
    let conn = pool.get().await?;
    let query = format!(
        "select * from ({SEARCH_UNION}) as s
         where {SEARCH_FILTER}
         order by s.created desc offset $3::int limit $4::int"
    );
    let mut results: Vec<SearchResult> = Vec::new();
    for row in conn
        .query(&query, &[&keyword, &kind, &offset, &limit])
        .await?
    {
        let category: String = row.get("category");
        let body: String = row.get("body");
        let username: String = row.get("username");
        let created: NaiveDateTime = row.get("created");
        results.push(SearchResult {
            path: row.get("path"),
            title: row.get("title"),
            body: body.chars().take(100).collect(),
            is_comment: row.get("is_comment"),
            is_tweet: category == "tweet",
            category,
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
    Ok(results)
}

pub async fn search_count(pool: &PgPool, keyword: &str, kind: &str) -> anyhow::Result<i32> {
    let conn = pool.get().await?;
    let query = format!(
        "select count(*)::int as count from ({SEARCH_UNION}) as s
         where {SEARCH_FILTER}"
    );
    let row = conn.query_one(&query, &[&keyword, &kind]).await?;
    Ok(row.get("count"))
}
