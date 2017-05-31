use postgres::error::Error;
use db;
use models;

#[derive(Serialize, Debug, Default)]
pub struct Post {
    id: i32,
    pub user_id: i32,
    title: String,
    body: String,
    user: models::user::User,
}

pub fn create(conn: db::PostgresConnection, user_id: i32, title: String, body: String) -> Result<(i32), Error> {
    let mut id = 0;
    for row in &conn.query("INSERT INTO posts (user_id, title, body) VALUES ($1, $2, $3) returning id;", &[&user_id, &title, &body]).unwrap() {
        id = row.get("id");
    }
    Ok(id)
}

pub fn list(conn: db::PostgresConnection) -> Result<Vec<Post>, Error> {
    let mut posts: Vec<Post> = Vec::new();
    for row in &conn.query("SELECT p.id, p.user_id, p.title, p.body, u.email, u.username, u.icon_url from posts as p join users as u on u.id = p.user_id", &[]).unwrap() {
        posts.push(Post {
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
    Ok(posts)
}

pub fn update(conn: db::PostgresConnection, id: i32, title: String, body: String) -> Result<(), Error> {
    conn.execute(
        "UPDATE posts set title = $1, body = $2 WHERE id = $3", &[&title, &body, &id]
    ).map(|_| ())
}

pub fn get_by_id(conn: db::PostgresConnection, id: i32) -> Result<Post, Error> {
    let rows = &conn.query("SELECT p.id, p.user_id, p.title, p.body, u.email, u.username, u.icon_url from posts as p join users as u on u.id = p.user_id where p.id = $1", &[&id]).unwrap();
    let row = rows.get(0);
    let post = Post {
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
    Ok(post)
}

pub fn get_marked_by_id(conn: db::PostgresConnection, id: i32) -> Result<Post, Error> {
    let rows = &conn.query("SELECT p.id, p.user_id, p.title, p.body, u.email, u.username, u.icon_url from posts as p join users as u on u.id = p.user_id where p.id = $1", &[&id]).unwrap();
    let row = rows.get(0);
    let mut post = Post {
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
    post.body = post.body.replace("\r\n", "\\n\\n");
    Ok(post)
}

pub fn delete_by_id(conn: db::PostgresConnection, id: i32) -> Result<(), Error> {
    conn.execute(
        "DELETE FROM posts WHERE id = $1;",
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

pub fn add_comment(conn: db::PostgresConnection, user_id: i32, post_id: i32, body: String) -> Result<(), Error> {
    conn.execute(
        "INSERT INTO post_comments (user_id, post_id, body) VALUES ($1, $2, $3);",
        &[&user_id, &post_id, &body]
    ).map(|_| ())
}

pub fn get_comments_by_post_id(conn: db::PostgresConnection, id: i32) -> Result<Vec<Comment>, Error> {
    let mut comments: Vec<Comment> = Vec::new();
    for row in &conn.query("SELECT c.id, c.user_id, c.post_id, c.body, u.email, u.username, u.icon_url from post_comments as c join users as u on u.id = c.user_id where c.post_id = $1", &[&id]).unwrap() {
        comments.push(Comment {
            id: row.get("id"),
            user_id: row.get("user_id"),
            post_id: row.get("post_id"),
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
