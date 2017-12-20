use postgres::error::Error;
use db;
use models;
use helper;
use chrono::{NaiveDateTime};

#[derive(Serialize, Debug)]
pub struct Gist {
    pub id: i32,
    pub user_id: i32,
    pub description: String,
    pub filename: String,
    pub code: String,
    pub created: NaiveDateTime,
    pub user: models::user::User,
}

pub fn create(conn: &db::PostgresConnection, user_id: &i32, description: &String, filename: &String, code: &String) -> Result<(i32), Error> {
    let mut gist_id = 0;
    for row in &conn.query("
        INSERT INTO gists (user_id, description, filename, code)
        VALUES ($1, $2, $3, $4) returning id;",
        &[&user_id, &description, &filename, &code]).unwrap() {
        gist_id = row.get("id");
    }
    Ok(gist_id)
}

pub fn list(conn: &db::PostgresConnection, offset: &i32, limit: &i32) -> Result<Vec<Gist>, Error> {
    let mut gists: Vec<Gist> = Vec::new();
    for row in &conn.query("
        SELECT g.id, g.user_id, g.description, g.filename, g.code, g.created, u.username, u.icon_url
        from gists as g
        join users as u on u.id = g.user_id
        order by g.id desc offset $1::int limit $2::int", &[&offset, &limit]).unwrap() {
        gists.push(Gist {
            id: row.get("id"),
            user_id: row.get("user_id"),
            description: row.get("description"),
            filename: row.get("filename"),
            code: row.get("code"),
            created: row.get("created"),
            user: models::user::User{
                id: row.get("user_id"),
                username: row.get("username"),
                icon_url: row.get("icon_url"),
                username_hash: helper::username_hash(row.get("username")),
            },
        });
    }
    Ok(gists)
}

pub fn count(conn: &db::PostgresConnection) -> Result<i32, Error> {
    let rows = &conn.query("SELECT count(*)::int as count from gists", &[]).unwrap();
    let row = rows.get(0);
    let count = row.get("count");
    Ok(count)
}

pub fn get_by_id(conn: &db::PostgresConnection, id: &i32) -> Result<Gist, Error> {
    let rows = &conn.query("SELECT g.id, g.user_id, g.description, g.filename, g.code, g.created, u.username, u.icon_url 
                            from gists as g join users as u on u.id=g.user_id 
                            where g.id = $1", &[&id]).unwrap();
    let row = rows.get(0);
    let gist = Gist {
        id: row.get("id"),
        user_id: row.get("user_id"),
        description: row.get("description"),
        filename: row.get("filename"),
        code: row.get("code"),
        created: row.get("created"),
        user: models::user::User{
            id: row.get("user_id"),
            username: row.get("username"),
            icon_url: row.get("icon_url"),
            username_hash: helper::username_hash(row.get("username")),
        },
    };
    Ok(gist)
}

pub fn update(conn: &db::PostgresConnection, id: &i32, description: &String, filename: &String, code: &String) -> Result<(), Error> {
    conn.execute(
        "UPDATE gists set description = $1, filename = $2, code = $3 WHERE id = $4", &[&description, &filename, &code, &id]
    ).unwrap();
    Ok(())
}

pub fn delete_by_id(conn: &db::PostgresConnection, id: &i32) -> Result<(), Error> {
    conn.execute(
        "DELETE FROM gists WHERE id = $1",
        &[&id]
    ).map(|_| ())
}

#[derive(Serialize, Debug, Default)]
pub struct Comment {
    pub id: i32,
    pub user_id: i32,
    pub gist_id: i32,
    pub body: String,
    pub user: models::user::User,
}

pub fn add_comment(conn: &db::PostgresConnection, user_id: &i32, gist_id: &i32, body: &String) -> Result<(), Error> {
    conn.execute(
        "INSERT INTO gist_comments (user_id, gist_id, body) VALUES ($1, $2, $3);",
        &[&user_id, &gist_id, &body]
    ).map(|_| ())
}

pub fn get_comments_by_gist_id(conn: &db::PostgresConnection, id: &i32) -> Result<Vec<Comment>, Error> {
    let mut comments: Vec<Comment> = Vec::new();
    for row in &conn.query("SELECT c.id, c.user_id, c.gist_id, c.body, u.username, u.icon_url from gist_comments as c join users as u on u.id = c.user_id where c.gist_id = $1 order by id asc", &[&id]).unwrap() {
        comments.push(Comment {
            id: row.get("id"),
            user_id: row.get("user_id"),
            gist_id: row.get("gist_id"),
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

pub fn get_comment_by_id(conn: &db::PostgresConnection, id: &i32) -> Result<Comment, Error> {
    let rows = &conn.query("SELECT c.*, u.username, u.icon_url from gist_comments as c join users as u on u.id = c.user_id where c.id = $1", &[&id]).unwrap();
    let row = rows.get(0);
    let comment = Comment {
        id: row.get("id"),
        user_id: row.get("user_id"),
        gist_id: row.get("gist_id"),
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
        "UPDATE gist_comments set body = $1 WHERE id = $2", &[&body, &id]
    ).unwrap();
    Ok(())
}

pub fn delete_comment_by_id(conn: &db::PostgresConnection, id: &i32) -> Result<(), Error> {
    conn.execute(
        "DELETE FROM gist_comments WHERE id = $1", &[&id]
    ).unwrap();
    Ok(())
}
