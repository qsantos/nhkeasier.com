use std::sync::Arc;

use askama_axum::Template;
use axum::{
    extract,
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
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

struct State {
    pool: sqlx::Pool<sqlx::Sqlite>,
    sub_edict_creator: SubEdictCreator,
    sub_enamdict_creator: SubEnamdictCreator,
}

async fn story(
    extract::State(state): extract::State<Arc<State>>,
    extract::Path(id): extract::Path<i64>,
) -> impl IntoResponse {
    let row = sqlx::query("SELECT * FROM nhkeasier_story WHERE id = $1")
        .bind(id)
        .fetch_one(&state.pool)
        .await
        .unwrap();
    let story = Story::from_row(&row).unwrap();

    let edict = story
        .content
        .as_ref()
        .map(|content| state.sub_edict_creator.from(content).join("\n"));
    let enamdict = story
        .content
        .as_ref()
        .map(|content| state.sub_enamdict_creator.from(content).join("\n"));

    Html(
        MyTemplate {
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
        .unwrap(),
    )
}

#[tokio::main]
async fn main() {
    let state = Arc::new(State {
        pool: SqlitePoolOptions::new()
            .max_connections(5)
            .connect("db.sqlite3")
            .await
            .unwrap(),
        sub_edict_creator: SubEdictCreator::from_files(),
        sub_enamdict_creator: SubEnamdictCreator::from_files(),
    });

    // build our application with a single route
    let app = Router::new()
        .route("/story/:id/", get(story))
        .with_state(state);

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
