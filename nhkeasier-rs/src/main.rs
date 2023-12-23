use askama::Template;
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::types::chrono::NaiveDateTime;
use sqlx::FromRow;

use edict2::{SubEdictCreator, SubEnamdictCreator};

#[derive(Clone, Debug, FromRow)]
#[allow(dead_code)]
struct Story<'a> {
    id: i64,
    story_id: &'a str,
    published: NaiveDateTime,
    title_with_ruby: &'a str,
    title: &'a str,
    content_with_ruby: Option<&'a str>,
    content: Option<&'a str>,
    image: Option<&'a str>,
    voice: Option<&'a str>,
    video_original: Option<&'a str>,
    video_reencoded: Option<&'a str>,
    subedict_created: bool,
    webpage: Option<&'a str>,
}

#[derive(Template)]
#[template(path = "story.html")]
struct MyTemplate<'a> {
    debug: bool,
    title: &'a str,
    description: Option<&'a str>,
    image: Option<&'a str>,
    player: Option<&'a str>,
    header: &'a str,
    story: &'a Story<'a>,
    previous_story_id: Option<i64>,
    next_story_id: Option<i64>,
    edict: Option<&'a str>,
    enamdict: Option<&'a str>,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), sqlx::Error> {
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect("db.sqlite3")
        .await?;

    let sub_edict_creator = SubEdictCreator::from_files();
    let sub_enamdict_creator = SubEnamdictCreator::from_files();

    let row = sqlx::query("SELECT * FROM nhkeasier_story ORDER BY id DESC")
        .fetch_one(&pool)
        .await?;
    let story = Story::from_row(&row)?;

    let edict = story
        .content
        .as_ref()
        .map(|content| sub_edict_creator.from(content).join("\n"));
    let enamdict = story
        .content
        .as_ref()
        .map(|content| sub_enamdict_creator.from(content).join("\n"));
    let t = MyTemplate {
        debug: true,
        title: story.title,
        description: story.content,
        image: story.image,
        player: None,
        header: "Single Story",
        previous_story_id: Some(story.id - 1),
        next_story_id: None,
        edict: edict.as_deref(),
        enamdict: enamdict.as_deref(),
        story: &story,
    }
    .render()
    .unwrap();

    println!("{t}");

    Ok(())
}
