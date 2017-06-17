use postgres::error::Error;
use db;
use models;
use helper;

#[derive(Serialize, Debug, Default)]
pub struct Post {
    pub id: i32,
    pub kind: String,
    pub user_id: i32,
    pub title: String,
    pub body: String,
    pub user: models::user::User,
    pub tags: Vec<models::tag::Tag>,
}

pub fn create(conn: &db::PostgresConnection, kind: &str, user_id: &i32, title: &String, body: &String, tags: &String) -> Result<(i32), Error> {
    let mut post_id = 0;
    for row in &conn.query("
        INSERT INTO posts (kind, user_id, title, body)
        VALUES ($1, $2, $3, $4) returning id;",
        &[&kind, &user_id, &title, &body]).unwrap() {
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
                    println!("Errored: {:?}", e);
                }
            }
        }
    }
    Ok(post_id)
}

pub fn list(conn: &db::PostgresConnection, kind: &str, offset: &i32, limit: &i32) -> Result<Vec<Post>, Error> {
    let mut posts: Vec<Post> = Vec::new();
    for row in &conn.query("
        SELECT p.id, p.kind, p.user_id, p.title, p.body, u.username, u.icon_url
        from posts as p
        join users as u on u.id = p.user_id
        where p.kind = $1
        order by p.id desc offset $2::int limit $3::int", &[&kind, &offset, &limit]).unwrap() {
        match models::tag::get_tags_by_post_id(&conn, &row.get("id")) {
            Ok(tags) => {
                posts.push(Post {
                    id: row.get("id"),
                    kind: row.get("kind"),
                    user_id: row.get("user_id"),
                    title: row.get("title"),
                    body: row.get("body"),
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
                println!("Errored: {:?}", e);
            }
        }
    }
    Ok(posts)
}

pub fn count(conn: &db::PostgresConnection, kind: &str) -> Result<i32, Error> {
    let rows = &conn.query("SELECT count(*)::int as count from posts where kind = $1", &[&kind]).unwrap();
    let row = rows.get(0);
    let count = row.get("count");
    Ok(count)
}

use super::tag;
pub fn update(conn: &db::PostgresConnection, id: &i32, title: &String, body: &String, tags: &String) -> Result<(), Error> {
    conn.execute(
        "UPDATE posts set title = $1, body = $2 WHERE id = $3", &[&title, &body, &id]
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
    let rows = &conn.query("SELECT p.id, p.kind, p.user_id, p.title, p.body, u.username, u.icon_url from posts as p join users as u on u.id=p.user_id where p.id = $1", &[&id]).unwrap();
    let row = rows.get(0);
    match models::tag::get_tags_by_post_id(&conn, &row.get("id")) {
        Ok(tags) => {
            let post = Post {
                id: row.get("id"),
                kind: row.get("kind"),
                user_id: row.get("user_id"),
                title: row.get("title"),
                body: row.get("body"),
                user: models::user::User{
                    id: row.get("user_id"),
                    username: row.get("username"),
                    icon_url: row.get("icon_url"),
                    username_hash: helper::username_hash(row.get("username")),
                },
                tags: tags,
            };
            Ok(post)
        },
        Err(e) => {
            println!("Errored: {:?}", e);
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
    id: i32,
    user_id: i32,
    post_id: i32,
    body: String,
    user: models::user::User,
}

pub fn add_comment(conn: &db::PostgresConnection, user_id: &i32, post_id: &i32, body: &String) -> Result<(), Error> {
    conn.execute(
        "INSERT INTO post_comments (user_id, post_id, body) VALUES ($1, $2, $3);",
        &[&user_id, &post_id, &body]
    ).map(|_| ())
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

pub fn count_all(conn: &db::PostgresConnection) -> Result<i32, Error> {
    let rows = &conn.query("SELECT count(*)::int as count from posts", &[]).unwrap();
    let row = rows.get(0);
    let count = row.get("count");
    Ok(count)
}


#[derive(Serialize, Debug, Default)]
pub struct Feed {
    id: i32,
    pub kind: String,
    pub user_id: i32,
    title: String,
    body: String,
    user: models::user::User,
    tags: Vec<models::tag::Tag>,
}

pub fn get_feeds(conn: &db::PostgresConnection, offset: &i32, limit: &i32) -> Result<Vec<Feed>, Error> {
    let mut feeds: Vec<Feed> = Vec::new();
    for row in &conn.query("
        (select p.id, p.kind, p.user_id, p.title, '' as body, u.username, u.icon_url, p.created from posts as p join users as u on u.id=p.user_id)
        union
        (select c.post_id, p.kind, c.user_id, p.title as title, c.body, u.username, u.icon_url, c.created from post_comments as c join users as u on u.id=c.user_id join posts as p on c.post_id=p.id)
        order by created desc offset $1::int limit $2::int", &[&offset, &limit]).unwrap() {
        match models::tag::get_tags_by_post_id(&conn, &row.get("id")) {
            Ok(tags) => {
                feeds.push(Feed {
                    id: row.get("id"),
                    kind: row.get("kind"),
                    user_id: row.get("user_id"),
                    title: row.get("title"),
                    body: row.get("body"),
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
                println!("Errored: {:?}", e);
            }
        }
    }
    Ok(feeds)
}

pub fn search(conn: &db::PostgresConnection, keyword: &String, offset: &i32, limit: &i32) -> Result<Vec<Post>, Error> {
    let mut posts: Vec<Post> = Vec::new();
    for row in &conn.query("SELECT p.id, p.kind, p.user_id, p.title, p.body, u.username, u.icon_url from posts as p join users as u on u.id = p.user_id where p.title like '%' || $1 || '%' or p.body like '%' || $1 || '%' order by p.id desc offset $2::int limit $3::int", &[&keyword, &offset, &limit]).unwrap() {
        match models::tag::get_tags_by_post_id(&conn, &row.get("id")) {
            Ok(tags) => {
                posts.push(Post {
                    id: row.get("id"),
                    kind: row.get("kind"),
                    user_id: row.get("user_id"),
                    title: row.get("title"),
                    body: row.get("body"),
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
                println!("Errored: {:?}", e);
            }
        }
    }
    Ok(posts)
}

pub fn search_count(conn: &db::PostgresConnection, keyword: &String) -> Result<i32, Error> {
    let rows = &conn.query("SELECT count(*)::int as count from posts where title like '%' || $1 || '%' or body like '%' || $1 || '%'", &[&keyword]).unwrap();
    let row = rows.get(0);
    let count = row.get("count");
    Ok(count)
}
