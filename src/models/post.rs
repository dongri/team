use postgres::error::Error;
use db;
use models;
use helper;
use chrono::{NaiveDateTime};

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

pub fn create(conn: &db::PostgresConnection, kind: &str, user_id: &i32, action: &String, title: &String, body: &String, tags: &String) -> Result<(i32), Error> {
    let mut post_id = 0;
    for row in &conn.query("
        INSERT INTO posts (kind, user_id, title, body, status)
        VALUES ($1, $2, $3, $4, $5) returning id;",
        &[&kind, &user_id, &title, &body, &action]).unwrap() {
        post_id = row.get("id");
    }
    for mut tag in tags.split(",") {
        tag = tag.trim();
        if tag != "" {
            match models::tag::select_or_create_tag_id(&conn, tag) {
                Ok(tag_id) => {
                    &conn.query("INSERT INTO taggings (tag_id, post_id) VALUES ($1, $2);", &[&tag_id, &post_id]).unwrap();
                },
                Err(e) => {
                    error!("Errored: {:?}", e);
                }
            }
        }
    }
    Ok(post_id)
}

pub fn list(conn: &db::PostgresConnection, kind: &str, offset: &i32, limit: &i32) -> Result<Vec<Post>, Error> {
    let mut posts: Vec<Post> = Vec::new();
    for row in &conn.query("
        SELECT p.id, p.kind, p.user_id, p.title, p.body, p.created, p.shared, p.status, u.username, u.icon_url
        from posts as p
        join users as u on u.id = p.user_id
        where p.status = 'publish' and p.kind = $1
        order by p.id desc offset $2::int limit $3::int", &[&kind, &offset, &limit]).unwrap() {
        match models::tag::get_tags_by_post_id(&conn, &row.get("id")) {
            Ok(tags) => {
                let mut post = Post {
                    id: row.get("id"),
                    kind: row.get("kind"),
                    user_id: row.get("user_id"),
                    title: row.get("title"),
                    body: row.get("body"),
                    created: row.get("created"),
                    formated_created: "".to_string(),
                    shared: row.get("shared"),
                    status: row.get("status"),
                    user: models::user::User{
                        id: row.get("user_id"),
                        username: row.get("username"),
                        icon_url: row.get("icon_url"),
                        username_hash: helper::username_hash(row.get("username")),
                    },
                    tags: tags,
                };
                post.formated_created = helper::jst_time_formatter(post.created);
                posts.push(post);
            },
            Err(e) => {
                error!("Errored: {:?}", e);
            }
        }
    }
    Ok(posts)
}

pub fn count(conn: &db::PostgresConnection, kind: &str) -> Result<i32, Error> {
    let rows = &conn.query("SELECT count(*)::int as count from posts where status = 'publish' and kind = $1", &[&kind]).unwrap();
    let row = rows.get(0);
    let count = row.get("count");
    Ok(count)
}

use super::tag;
pub fn update(conn: &db::PostgresConnection, id: &i32, title: &String, body: &String, tags: &String, action: &String) -> Result<(), Error> {
    conn.execute(
        "UPDATE posts set title = $1, body = $2, status = $3 WHERE id = $4", &[&title, &body, &action, &id]
    ).unwrap();
    let mut old_tag_ids: Vec<i32> = models::tag::get_tags_by_post_id(&conn, &id)
        .or::<Vec<tag::Tag>>(Ok(Vec::<tag::Tag>::new()))
        .unwrap().into_iter().map(|t|t.id).collect();

    let mut new_tag_ids:Vec<i32> = tags.split(",").filter_map::<i32, _>(|tag| {
        let tag = tag.trim();
        if tag == "" {
            return None;
        }
        return models::tag::select_or_create_tag_id(&conn, tag)
            .or_else(|e| Err(e))
            .ok().or(None)
            .and_then(|tag_id|{
                if old_tag_ids.contains(&tag_id) {
                    old_tag_ids.iter().position(|r| r == &tag_id).and_then(|index|{
                        old_tag_ids.remove(index);
                        return Some(index);
                    });
                    return None;
                }
                return Some(tag_id);
            });
        }).collect();
    new_tag_ids.dedup();
    for tag_id in  new_tag_ids.iter() {
        &conn.query(r#"INSERT INTO taggings (tag_id, post_id)
            SELECT $1, $2
            WHERE NOT EXISTS (SELECT * FROM taggings WHERE tag_id = $3 AND post_id = $4)"#,
            &[&tag_id, &id, &tag_id, &id]).unwrap();
    }
    for tag_id in &old_tag_ids {
        &conn.query("DELETE FROM taggings where tag_id = $1 and post_id = $2;", &[&tag_id, &id]).unwrap();
    }
    Ok(())
}

pub fn get_by_id(conn: &db::PostgresConnection, id: &i32) -> Result<Post, Error> {
    let rows = &conn.query("SELECT p.id, p.kind, p.user_id, p.title, p.body, p.created, p.shared, p.status, u.username, u.icon_url from posts as p join users as u on u.id=p.user_id where p.id = $1", &[&id]).unwrap();
    let row = rows.get(0);
    match models::tag::get_tags_by_post_id(&conn, &row.get("id")) {
        Ok(tags) => {
            let mut post = Post {
                id: row.get("id"),
                kind: row.get("kind"),
                user_id: row.get("user_id"),
                title: row.get("title"),
                body: row.get("body"),
                created: row.get("created"),
                formated_created: "".to_string(),
                shared: row.get("shared"),
                status: row.get("status"),
                user: models::user::User{
                    id: row.get("user_id"),
                    username: row.get("username"),
                    icon_url: row.get("icon_url"),
                    username_hash: helper::username_hash(row.get("username")),
                },
                tags: tags,
            };
            post.formated_created = helper::jst_time_formatter(post.created);
            Ok(post)
        },
        Err(e) => {
            error!("Errored: {:?}", e);
            Err(e)
        }
    }
}

pub fn delete_by_id(conn: &db::PostgresConnection, id: &i32) -> Result<(), Error> {
    conn.execute(
        "DELETE FROM posts WHERE id = $1",
        &[&id]
    ).map(|_| ())
}

#[derive(Serialize, Debug, Default)]
pub struct Comment {
    pub id: i32,
    pub user_id: i32,
    pub post_id: i32,
    pub body: String,
    pub user: models::user::User,
}

pub fn add_comment(conn: &db::PostgresConnection, user_id: &i32, post_id: &i32, body: &String) -> Result<(i32), Error> {
    &conn.query("
        INSERT INTO post_comments (user_id, post_id, body) 
        VALUES ($1, $2, $3) returning id;",
        &[&user_id, &post_id, &body]).unwrap();
    let posts = &conn.query("SELECT * from posts where id = $1", &[&post_id]).unwrap();
    let post = posts.get(0);
    let post_user_id: i32 = post.get("user_id");
    models::notification::create(conn, &format!("/post/show/{}", post_id).to_string(), user_id, &post_user_id, body)
}

pub fn get_comments_by_post_id(conn: &db::PostgresConnection, id: &i32) -> Result<Vec<Comment>, Error> {
    let mut comments: Vec<Comment> = Vec::new();
    for row in &conn.query("SELECT c.id, c.user_id, c.post_id, c.body, u.username, u.icon_url from post_comments as c join users as u on u.id = c.user_id where c.post_id = $1 order by id asc", &[&id]).unwrap() {
        comments.push(Comment {
            id: row.get("id"),
            user_id: row.get("user_id"),
            post_id: row.get("post_id"),
            body: row.get("body"),
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

#[derive(Serialize, Debug)]
pub struct Feed {
    id: i32,
    pub post: String,
    pub comment: String,
    pub gist: String,
    pub kind: String,
    pub user_id: i32,
    title: String,
    body: String,
    created: NaiveDateTime,
    formated_created: String,
    user: models::user::User,
    tags: Vec<models::tag::Tag>,
}

pub fn get_feeds(conn: &db::PostgresConnection, offset: &i32, limit: &i32) -> Result<Vec<Feed>, Error> {
    let mut feeds: Vec<Feed> = Vec::new();
    for row in &conn.query("
        (select '' as comment, '' as gist, 'post' as post, p.id, p.kind, p.user_id, p.title, '' as body, u.username, u.icon_url, p.created from posts as p join users as u on u.id=p.user_id where p.status = 'publish')
        union
        (select 'comment' as comment, '' as gist, '' as post,  c.post_id, p.kind, c.user_id, p.title as title, c.body, u.username, u.icon_url, c.created from post_comments as c join users as u on u.id=c.user_id join posts as p on c.post_id=p.id)
        union
        (select  '' as comment, 'gist' as gist, '' as post, g.id, '' as kind, g.user_id, g.filename as title, '' as body, u.username, u.icon_url, g.created from gists as g join users as u on u.id=g.user_id)
        order by created desc offset $1::int limit $2::int", &[&offset, &limit]).unwrap() {
        match models::tag::get_tags_by_post_id(&conn, &row.get("id")) {
            Ok(tags) => {
                let mut body: String = row.get("body");
                body = body.as_str().chars().skip(0).take(50).collect();
                let mut feed = Feed {
                    id: row.get("id"),
                    post: row.get("post"),
                    comment: row.get("comment"),
                    gist: row.get("gist"),
                    kind: row.get("kind"),
                    user_id: row.get("user_id"),
                    title: row.get("title"),
                    body: body,
                    created: row.get("created"),
                    formated_created: "".to_string(),
                    user: models::user::User{
                        id: row.get("user_id"),
                        username: row.get("username"),
                        icon_url: row.get("icon_url"),
                        username_hash: helper::username_hash(row.get("username")),
                    },
                    tags: tags,
                };
                feed.formated_created = helper::jst_time_formatter(feed.created);
                feeds.push(feed);
            },
            Err(e) => {
                error!("Errored: {:?}", e);
            }
        }
    }
    Ok(feeds)
}

pub fn get_feed_count(conn: &db::PostgresConnection) -> Result<i32, Error> {
    let rows = &conn.query("
    select sum(count)::int as count from
    (select count(*) from posts where status = 'publish'
    union all
    select count(*) as b from post_comments) as t;
    ", &[]).unwrap();
    let row = rows.get(0);
    let count = row.get("count");
    Ok(count)
}

pub fn search(conn: &db::PostgresConnection, keyword: &String, kind: &String, offset: &i32, limit: &i32) -> Result<Vec<Post>, Error> {
    let mut kind_param = String::from("");
    if kind != "all" {
        kind_param = format!("{}", kind);
    }
    let mut posts: Vec<Post> = Vec::new();
    for row in &conn.query("
        SELECT p.id, p.kind, p.user_id, p.title, p.body, p.created, p.shared, p.status, u.username, u.icon_url from posts as p
        join users as u on u.id = p.user_id
        where p.status = 'publish' and (p.title ilike '%' || $1 || '%' or p.body ilike '%' || $1 || '%') and p.kind like '%' || $2 || '%'
        order by p.id desc offset $3::int limit $4::int", &[&keyword, &kind_param, &offset, &limit]).unwrap() {
        match models::tag::get_tags_by_post_id(&conn, &row.get("id")) {
            Ok(tags) => {
                let mut post = Post {
                    id: row.get("id"),
                    kind: row.get("kind"),
                    user_id: row.get("user_id"),
                    title: row.get("title"),
                    body: row.get("body"),
                    created: row.get("created"),
                    formated_created: "".to_string(),
                    shared: row.get("shared"),
                    status: row.get("status"),
                    user: models::user::User{
                        id: row.get("user_id"),
                        username: row.get("username"),
                        icon_url: row.get("icon_url"),
                        username_hash: helper::username_hash(row.get("username")),
                    },
                    tags: tags,
                };
                post.formated_created = helper::jst_time_formatter(post.created);
                posts.push(post);
            },
            Err(e) => {
                error!("Errored: {:?}", e);
            }
        }
    }
    Ok(posts)
}

pub fn search_count(conn: &db::PostgresConnection, keyword: &String, kind: &String) -> Result<i32, Error> {
    let mut kind_param = String::from("");
    if kind != "all" {
        kind_param = format!("{}", kind);
    }
    let rows = &conn.query("
        SELECT count(*)::int as count from posts where status = 'publish' and (title ilike '%' || $1 || '%' or body ilike '%' || $1 || '%') and kind like '%' || $2 || '%'", &[&keyword, &kind_param]).unwrap();
    let row = rows.get(0);
    let count = row.get("count");
    Ok(count)
}

pub fn stock_post(conn: &db::PostgresConnection, user_id: &i32, post_id: &i32) -> Result<(), Error> {
    conn.execute(
        "INSERT INTO stocks (user_id, post_id) VALUES ($1, $2);",
        &[&user_id, &post_id]
    ).map(|_| ())
}


pub fn stocked_list(conn: &db::PostgresConnection, user_id: &i32, offset: &i32, limit: &i32) -> Result<Vec<Post>, Error> {
    let mut posts: Vec<Post> = Vec::new();
    for row in &conn.query("
        SELECT p.id, p.kind, p.user_id, p.title, p.body, p.created, p.shared, p.status, u.username, u.icon_url
        from posts as p
        join stocks as s on s.post_id = p.id
        join users as u on u.id = p.user_id
        where s.user_id = $1
        order by s.id desc offset $2::int limit $3::int", &[&user_id, &offset, &limit]).unwrap() {
        match models::tag::get_tags_by_post_id(&conn, &row.get("id")) {
            Ok(tags) => {
                let mut post = Post {
                    id: row.get("id"),
                    kind: row.get("kind"),
                    user_id: row.get("user_id"),
                    title: row.get("title"),
                    body: row.get("body"),
                    created: row.get("created"),
                    formated_created: "".to_string(),
                    shared: row.get("shared"),
                    status: row.get("status"),
                    user: models::user::User{
                        id: row.get("user_id"),
                        username: row.get("username"),
                        icon_url: row.get("icon_url"),
                        username_hash: helper::username_hash(row.get("username")),
                    },
                    tags: tags,
                };
                post.formated_created = helper::jst_time_formatter(post.created);
                posts.push(post);
            },
            Err(e) => {
                error!("Errored: {:?}", e);
            }
        }
    }
    Ok(posts)
}

pub fn stocked_count(conn: &db::PostgresConnection, user_id: &i32) -> Result<i32, Error> {
    let rows = &conn.query("SELECT count(*)::int as count from stocks where user_id = $1", &[&user_id]).unwrap();
    let row = rows.get(0);
    let count = row.get("count");
    Ok(count)
}

pub fn is_stocked(conn: &db::PostgresConnection, user_id: &i32, post_id: &i32) -> Result<bool, Error> {
    let rows = &conn.query("SELECT count(*)::int as count from stocks where user_id = $1 and post_id = $2", &[&user_id, &post_id]).unwrap();
    let row = rows.get(0);
    let count: i32 = row.get("count");
    let stocked = count > 0;
    Ok(stocked)
}

pub fn stock_remove(conn: &db::PostgresConnection, user_id: &i32, post_id: &i32) -> Result<(), Error> {
    conn.execute(
        "delete from stocks where user_id = $1 and post_id = $2",
        &[&user_id, &post_id]
    ).map(|_| ())
}

pub fn draft_list(conn: &db::PostgresConnection, user_id: &i32) -> Result<Vec<Post>, Error> {
    let mut posts: Vec<Post> = Vec::new();
    for row in &conn.query("
        SELECT p.id, p.kind, p.user_id, p.title, p.body, p.created, p.shared, p.status, u.username, u.icon_url
        from posts as p
        join users as u on u.id = p.user_id
        where p.status = 'draft' and p.user_id = $1
        order by p.id desc", &[&user_id]).unwrap() {
        match models::tag::get_tags_by_post_id(&conn, &row.get("id")) {
            Ok(tags) => {
                posts.push(Post {
                    id: row.get("id"),
                    kind: row.get("kind"),
                    user_id: row.get("user_id"),
                    title: row.get("title"),
                    body: row.get("body"),
                    created: row.get("created"),
                    formated_created: "".to_string(),
                    shared: row.get("shared"),
                    status: row.get("status"),
                    user: models::user::User{
                        id: row.get("user_id"),
                        username: row.get("username"),
                        icon_url: row.get("icon_url"),
                        username_hash: helper::username_hash(row.get("username")),
                    },
                    tags: tags,
                });
            },
            Err(e) => {
                error!("Errored: {:?}", e);
            }
        }
    }
    Ok(posts)
}

pub fn get_comment_by_id(conn: &db::PostgresConnection, id: &i32) -> Result<Comment, Error> {
    let rows = &conn.query("SELECT p.*, u.username, u.icon_url from post_comments as p join users as u on u.id = p.user_id where p.id = $1", &[&id]).unwrap();
    let row = rows.get(0);
    let comment = Comment {
        id: row.get("id"),
        user_id: row.get("user_id"),
        post_id: row.get("post_id"),
        body: row.get("body"),
        user: models::user::User{
            id: row.get("user_id"),
            username: row.get("username"),
            icon_url: row.get("icon_url"),
            username_hash: helper::username_hash(row.get("username")),
        },
    };
    Ok(comment)
}

pub fn update_comment_by_id(conn: &db::PostgresConnection, id: &i32, body: &String) -> Result<(), Error> {
    conn.execute(
        "UPDATE post_comments set body = $1 WHERE id = $2", &[&body, &id]
    ).unwrap();
    Ok(())
}

pub fn delete_comment_by_id(conn: &db::PostgresConnection, id: &i32) -> Result<(), Error> {
    conn.execute(
        "DELETE FROM post_comments WHERE id = $1", &[&id]
    ).unwrap();
    Ok(())
}

pub fn user_posts(conn: &db::PostgresConnection, username: &str, offset: &i32, limit: &i32, kind: &str) -> Result<Vec<Post>, Error> {
    let mut posts: Vec<Post> = Vec::new();
    for row in &conn.query("
        SELECT p.id, p.kind, p.user_id, p.title, p.body, p.created, p.shared, p.status, u.username, u.icon_url
        from posts as p
        join users as u on u.id = p.user_id
        where p.status = 'publish' and u.username = $1 and p.kind = $2
        order by p.id desc offset $3::int limit $4::int", &[&username, &kind, &offset, &limit]).unwrap() {
        match models::tag::get_tags_by_post_id(&conn, &row.get("id")) {
            Ok(tags) => {
                let mut post = Post {
                    id: row.get("id"),
                    kind: row.get("kind"),
                    user_id: row.get("user_id"),
                    title: row.get("title"),
                    body: row.get("body"),
                    created: row.get("created"),
                    formated_created: "".to_string(),
                    shared: row.get("shared"),
                    status: row.get("status"),
                    user: models::user::User{
                        id: row.get("user_id"),
                        username: row.get("username"),
                        icon_url: row.get("icon_url"),
                        username_hash: helper::username_hash(row.get("username")),
                    },
                    tags: tags,
                };
                post.formated_created = helper::jst_time_formatter(post.created);
                posts.push(post)
            },
            Err(e) => {
                error!("Errored: {:?}", e);
            }
        }
    }
    Ok(posts)
}

pub fn user_posts_count(conn: &db::PostgresConnection, username: &str, kind: &str) -> Result<i32, Error> {
    let rows = &conn.query("SELECT count(*)::int as count from posts as p join users as u on u.id=
    p.user_id where p.status = 'publish' and u.username = $1 and p.kind = $2", &[&username, &kind]).unwrap();
    let row = rows.get(0);
    let count = row.get("count");
    Ok(count)
}

pub fn share_post(conn: &db::PostgresConnection, post_id: &i32) -> Result<(), Error> {
    conn.execute(
        "update posts set shared = true where id = $1",
        &[&post_id]
    ).map(|_| ())
}
