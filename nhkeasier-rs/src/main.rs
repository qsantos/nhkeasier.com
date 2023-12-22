use sqlx::sqlite::SqlitePoolOptions;
use sqlx::types::chrono::{DateTime, Utc};
use sqlx::FromRow;

#[derive(Debug, FromRow)]
#[allow(dead_code)]
struct Story {
    id: u32,
    story_id: String,
    published: DateTime<Utc>,
    title_with_ruby: String,
    title: String,
    content_with_ruby: String,
    content: String,
    webpage: String,
    image: String,
    voice: String,
    video_original: String,
    video_reencoded: String,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), sqlx::Error> {
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect("db.sqlite3")
        .await?;

    let story = sqlx::query_as::<_, Story>("SELECT * FROM nhkeasier_story ORDER BY id DESC")
        .fetch_one(&pool)
        .await?;

    println!("{story:?}");

    Ok(())
}
