use sqlx::sqlite::SqlitePoolOptions;
use sqlx::types::chrono::NaiveDateTime;
use sqlx::FromRow;

use askama::Template;

#[derive(Debug, FromRow)]
#[allow(dead_code)]
struct Story {
    id: i64,
    story_id: String,
    published: NaiveDateTime,
    title_with_ruby: String,
    title: String,
    content_with_ruby: Option<String>,
    content: Option<String>,
    image: Option<String>,
    voice: Option<String>,
    video_original: Option<String>,
    video_reencoded: Option<String>,
    subedict_created: bool,
    webpage: Option<String>,
}

#[derive(Template)]
#[template(path = "story.html")]
struct MyTemplate {
    debug: bool,
    title: String,
    description: Option<String>,
    image: Option<String>,
    player: Option<String>,
    header: String,
    story: Story,
    previous_story_id: Option<i64>,
    next_story_id: Option<i64>,
    edict: String,
    enamdict: String,
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

    let t = MyTemplate {
        debug: true,
        title: story.title.clone(),
        description: story.content.clone(),
        image: story.image.clone(),
        player: None,
        header: "Single Story".to_string(),
        previous_story_id: Some(story.id - 1),
        next_story_id: None,
        story,
        edict: "".to_string(),
        enamdict: "".to_string(),
    };
    println!("{}", t.render().unwrap());

    Ok(())
}
