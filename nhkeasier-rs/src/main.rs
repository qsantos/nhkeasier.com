use sqlx::sqlite::SqlitePoolOptions;
use sqlx::types::chrono::NaiveDateTime;
use sqlx::FromRow;

#[derive(Debug, FromRow)]
#[allow(dead_code)]
struct Story {
    id: i64,
    story_id: Option<String>,
    published: Option<NaiveDateTime>,
    title_with_ruby: Option<String>,
    title: Option<String>,
    content_with_ruby: Option<String>,
    content: Option<String>,
    image: Option<String>,
    voice: Option<String>,
    video_original: Option<String>,
    video_reencoded: Option<String>,
    subedict_created: bool,
    webpage: Option<String>,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), sqlx::Error> {
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect("db.sqlite3")
        .await?;

    let story = sqlx::query_as!(Story, "SELECT * FROM nhkeasier_story ORDER BY id DESC")
        .fetch_one(&pool)
        .await?;

    println!("{story:?}");

    Ok(())
}
