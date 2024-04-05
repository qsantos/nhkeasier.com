use chrono::NaiveDateTime;
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::{FromRow, Pool, Sqlite};

#[derive(Clone, Debug, FromRow)]
#[allow(dead_code)]
pub struct Story<'a> {
    pub id: i64,
    pub news_id: &'a str,
    pub published: NaiveDateTime,
    pub title_with_ruby: &'a str,
    pub title: &'a str,
    pub content_with_ruby: Option<&'a str>,
    pub content: Option<&'a str>,
    pub image: Option<&'a str>,
    pub voice: Option<&'a str>,
    pub video_original: Option<&'a str>,
    pub video_reencoded: Option<&'a str>,
    pub subedict_created: bool,
    pub webpage: Option<&'a str>,
}

pub async fn connect_to_database() -> Pool<Sqlite> {
    let database_url =
        std::env::var("DATABASE_URL").expect("missing environment variable DATABASE_URL");
    SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("failed to connect to database")
}
