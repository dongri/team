use postgres::error::Error;
use db;
use models;
use helper;

#[derive(Serialize, Debug, Default)]
pub struct Tag {
    pub id: i32,
    pub name: String,
}

pub fn select_or_create_tag_id(conn: &db::PostgresConnection, tag_name: &str) -> Result<(i32), Error> {
    let rows = &conn.query("SELECT id from tags where name = $1", &[&tag_name]).unwrap();
    if rows.len() > 0 {
        let row = rows.get(0);
        let tag_id: i32 = row.get("id");
        return Ok(tag_id);
    }
    let rows = &conn.query("INSERT INTO tags (name) VALUES ($1) returning id;", &[&tag_name]).unwrap();
    let row = rows.get(0);
    let tag_id: i32 = row.get("id");
    Ok(tag_id)
}

pub fn get_tags_by_post_id(conn: &db::PostgresConnection, post_id: &i32) -> Result<Vec<Tag>, Error> {
    let mut tags: Vec<Tag> = Vec::new();
    for row in &conn.query("select t2.id, t2.name from taggings as t1 join tags as t2 on t1.tag_id = t2.id where t1.post_id = $1 order by t2.id desc", &[&post_id]).unwrap() {
        tags.push(Tag {
            id: row.get("id"),
            name: row.get("name"),
        });
    }
    Ok(tags)
}

pub fn tag_search(conn: &db::PostgresConnection, tag_name: &String, offset: i32, limit: i32) -> Result<Vec<models::post::Post>, Error> {
    let mut posts: Vec<models::post::Post> = Vec::new();
    for row in &conn.query("SELECT p.id, p.kind, p.user_id, p.title, p.body, p.created, u.username, u.icon_url from posts as p join users as u on u.id = p.user_id join taggings as t on p.id = t.post_id join tags as tg on t.tag_id = tg.id where tg.name = $1 order by p.id desc offset $2::int limit $3::int", &[&tag_name, &offset, &limit]).unwrap() {
        match models::tag::get_tags_by_post_id(&conn, &row.get("id")) {
            Ok(tags) => {
                posts.push(models::post::Post {
                    id: row.get("id"),
                    kind: row.get("kind"),
                    user_id: row.get("user_id"),
                    title: row.get("title"),
                    body: row.get("body"),
                    created: row.get("created"),
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

pub fn tag_count(conn: &db::PostgresConnection, tag_name: &String) -> Result<i32, Error> {
    let rows = &conn.query("SELECT count(p.*)::int as count from posts as p join taggings as t on p.id = t.post_id join tags tg on t.tag_id = tg.id where tg.name = $1", &[&tag_name]).unwrap();
    let row = rows.get(0);
    let count = row.get("count");
    Ok(count)
}
