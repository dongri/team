use postgres::error::Error;
use db;
use models;

#[derive(Serialize, Debug, Default)]
pub struct Nippo {
    id: i32,
    pub user_id: i32,
    title: String,
    body: String,
    user: models::user::User,
}

pub fn create(conn: db::PostgresConnection, user_id: i32, title: String, body: String) -> Result<(i32), Error> {
    let mut id = 0;
    for row in &conn.query("INSERT INTO nippos (user_id, title, body) VALUES ($1, $2, $3) returning id;", &[&user_id, &title, &body]).unwrap() {
        id = row.get("id");
    }
    Ok(id)
}

pub fn list(conn: db::PostgresConnection, offset: i32, limit: i32) -> Result<Vec<Nippo>, Error> {
    let mut nippos: Vec<Nippo> = Vec::new();
    for row in &conn.query("SELECT n.id, n.user_id, n.title, n.body, u.email, u.username, u.icon_url from nippos as n join users as u on u.id = n.user_id offset $1::int limit $2::int", &[&offset, &limit]).unwrap() {
        nippos.push(Nippo {
            id: row.get("id"),
            user_id: row.get("user_id"),
            title: row.get("title"),
            body: row.get("body"),
            user: models::user::User{
                id: row.get("user_id"),
                email: row.get("email"),
                username: row.get("username"),
                icon_url: row.get("icon_url"),
            }
        });
    }
    Ok(nippos)
}

pub fn count(conn: db::PostgresConnection) -> Result<i32, Error> {
    let rows = &conn.query("SELECT count(*)::int as count from nippos", &[]).unwrap();
    let row = rows.get(0);
    let count = row.get("count");
    Ok(count)
}


pub fn update(conn: db::PostgresConnection, id: i32, title: String, body: String) -> Result<(), Error> {
    conn.execute(
        "UPDATE nippos set title = $1, body = $2 WHERE id = $3", &[&title, &body, &id]
    ).map(|_| ())
}

pub fn get_by_id(conn: db::PostgresConnection, id: i32) -> Result<Nippo, Error> {
    let rows = &conn.query("SELECT n.id, n.user_id, n.title, n.body, u.email, u.username, u.icon_url from nippos as n join users as u on u.id=n.user_id where n.id = $1", &[&id]).unwrap();
    let row = rows.get(0);
    let nippo = Nippo {
        id: row.get("id"),
        user_id: row.get("user_id"),
        title: row.get("title"),
        body: row.get("body"),
        user: models::user::User{
            id: row.get("user_id"),
            email: row.get("email"),
            username: row.get("username"),
            icon_url: row.get("icon_url"),
        }
    };
    Ok(nippo)
}

pub fn get_marked_by_id(conn: db::PostgresConnection, id: i32) -> Result<Nippo, Error> {
    let rows = &conn.query("SELECT n.id, n.user_id, n.title, n.body, u.email, u.username, u.icon_url from nippos as n join users as u on u.id=n.user_id where n.id = $1", &[&id]).unwrap();
    let row = rows.get(0);
    let mut nippo = Nippo {
        id: row.get("id"),
        user_id: row.get("user_id"),
        title: row.get("title"),
        body: row.get("body"),
        user: models::user::User{
            id: row.get("user_id"),
            email: row.get("email"),
            username: row.get("username"),
            icon_url: row.get("icon_url"),
        }
    };
    nippo.body = nippo.body.replace("\r\n", "\\n\\n");
    Ok(nippo)
}

pub fn delete_by_id(conn: db::PostgresConnection, id: i32) -> Result<(), Error> {
    conn.execute(
        "DELETE FROM nippos WHERE id = $1;",
        &[&id]
    ).map(|_| ())
}

#[derive(Serialize, Debug, Default)]
pub struct Comment {
    id: i32,
    user_id: i32,
    nippo_id: i32,
    body: String,
    user: models::user::User,
}

pub fn add_comment(conn: db::PostgresConnection, user_id: i32, nippo_id: i32, body: String) -> Result<(), Error> {
    conn.execute(
        "INSERT INTO nippo_comments (user_id, nippo_id, body) VALUES ($1, $2, $3);",
        &[&user_id, &nippo_id, &body]
    ).map(|_| ())
}

pub fn get_comments_by_nippo_id(conn: db::PostgresConnection, id: i32) -> Result<Vec<Comment>, Error> {
    let mut comments: Vec<Comment> = Vec::new();
    for row in &conn.query("SELECT c.id, c.user_id, c.nippo_id, c.body, u.email, u.username, u.icon_url from nippo_comments as c join users as u on u.id = c.user_id where c.nippo_id = $1", &[&id]).unwrap() {
        comments.push(Comment {
            id: row.get("id"),
            user_id: row.get("user_id"),
            nippo_id: row.get("nippo_id"),
            body: row.get("body"),
            user: models::user::User{
                id: row.get("user_id"),
                email: row.get("email"),
                username: row.get("username"),
                icon_url: row.get("icon_url"),
            }
        });
    }
    Ok(comments)
}
