use postgres::error::Error;
use db;

#[derive(Serialize, Debug, Default)]
pub struct Nippo {
    id: i32,
    pub user_id: i32,
    title: String,
    body: String,
}

pub fn create_nippo(conn: db::PostgresConnection, user_id: i32, title: String, body: String) -> Result<(i32), Error> {
    let mut id = 0;
    for row in &conn.query("INSERT INTO nippos (user_id, title, body) VALUES ($1, $2, $3) returning id;", &[&user_id, &title, &body]).unwrap() {
        id = row.get("id");
    }
    Ok(id)
}

pub fn list_nippos(conn: db::PostgresConnection) -> Result<Vec<Nippo>, Error> {
    let mut nippos: Vec<Nippo> = Vec::new();
    for row in &conn.query("SELECT id, user_id, title, body from nippos", &[]).unwrap() {
        nippos.push(Nippo {
            id: row.get("id"),
            user_id: row.get("user_id"),
            title: row.get("title"),
            body: row.get("body"),
        });
    }
    Ok(nippos)
}

pub fn get_nippo_by_id(conn: db::PostgresConnection, id: i32) -> Result<Nippo, Error> {
    let mut nippo: Nippo = Nippo{..Default::default()};
    for row in &conn.query("SELECT id, user_id, title, body from nippos where id = $1", &[&id]).unwrap() {
        nippo = Nippo {
            id: row.get("id"),
            user_id: row.get("user_id"),
            title: row.get("title"),
            body: row.get("body"),
        };
    }
    Ok(nippo)
}

pub fn get_marked_nippo_by_id(conn: db::PostgresConnection, id: i32) -> Result<Nippo, Error> {
    let mut nippo: Nippo = Nippo{..Default::default()};
    for row in &conn.query("SELECT id, user_id, title, body from nippos where id = $1", &[&id]).unwrap() {
        nippo = Nippo {
            id: row.get("id"),
            user_id: row.get("user_id"),
            title: row.get("title"),
            body: row.get("body"),
        };
    }
    nippo.body = nippo.body.replace("\r\n", "\\n\\n");
    Ok(nippo)
}

pub fn delete_nippo_by_id(conn: db::PostgresConnection, id: i32) -> Result<(), Error> {
    conn.execute(
        "DELETE FROM nippos WHERE id = $1;",
        &[&id]
    ).map(|_| ())
}

pub fn update_nippo(conn: db::PostgresConnection, id: i32, title: String, body: String) -> Result<(), Error> {
    conn.execute(
        "UPDATE nippos set title = $1, body = $2 WHERE id = $3", &[&title, &body, &id]
    ).map(|_| ())
}

#[derive(Serialize, Debug, Default)]
pub struct Comment {
    id: i32,
    user_id: i32,
    nippo_id: i32,
    body: String,
}

pub fn add_comment_nippo(conn: db::PostgresConnection, user_id: i32, nippo_id: i32, body: String) -> Result<(), Error> {
    conn.execute(
        "INSERT INTO nippo_comments (user_id, nippo_id, body) VALUES ($1, $2, $3);",
        &[&user_id, &nippo_id, &body]
    ).map(|_| ())
}

pub fn get_nippo_comments(conn: db::PostgresConnection, id: i32) -> Result<Vec<Comment>, Error> {
    let mut comments: Vec<Comment> = Vec::new();
    for row in &conn.query("SELECT id, user_id, nippo_id, body from nippo_comments where nippo_id = $1", &[&id]).unwrap() {
        comments.push(Comment {
            id: row.get("id"),
            user_id: row.get("user_id"),
            nippo_id: row.get("nippo_id"),
            body: row.get("body"),
        });
    }
    Ok(comments)
}
