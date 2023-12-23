use std::borrow::Cow;
use std::sync::Arc;

use askama_axum::Template;
use axum::{
    extract,
    http::header,
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use lazy_static::lazy_static;
use regex::Regex;
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::types::chrono::{FixedOffset, NaiveDate, NaiveDateTime, TimeZone};
use sqlx::FromRow;
use tower_http::services::ServeDir;

use edict2::{SubEdictCreator, SubEnamdictCreator};

lazy_static! {
    static ref JST: FixedOffset = FixedOffset::east_opt(3600 * 9).unwrap();
    static ref REMOVE_HTML_REGEX: Regex = Regex::new("<.*?>").unwrap();
}

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
#[template(path = "message.html")]
struct MessageTemplate<'a> {
    debug: bool,
    title: &'a str,
    description: Option<&'a str>,
    image: Option<&'a str>,
    player: Option<&'a str>,
    header: &'a str,
    message: &'a str,
}

#[derive(Template)]
#[template(path = "about.html")]
struct AboutTemplate<'a> {
    debug: bool,
    title: &'a str,
    description: Option<&'a str>,
    image: Option<&'a str>,
    player: Option<&'a str>,
    header: &'a str,
}

#[derive(Template)]
#[template(path = "contact.html")]
struct ContactTemplate<'a> {
    debug: bool,
    title: &'a str,
    description: Option<&'a str>,
    image: Option<&'a str>,
    player: Option<&'a str>,
    header: &'a str,
}

#[derive(Template)]
#[template(path = "index.html")]
struct ArchiveTemplate<'a> {
    debug: bool,
    title: &'a str,
    description: Option<&'a str>,
    image: Option<&'a str>,
    player: Option<&'a str>,
    header: &'a str,
    stories: Vec<Story<'a>>,
    previous_day: Option<NaiveDate>,
    date: NaiveDate,
    next_day: Option<NaiveDate>,
    edict: Option<&'a str>,
    enamdict: Option<&'a str>,
}

#[derive(Template)]
#[template(path = "story.html")]
struct StoryTemplate<'a> {
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

#[derive(Template)]
#[template(path = "feed.atom", escape = "xml")]
struct FeedTemplate<'a> {
    stories: Vec<Story<'a>>,
}

struct State {
    pool: sqlx::Pool<sqlx::Sqlite>,
    sub_edict_creator: SubEdictCreator,
    sub_enamdict_creator: SubEnamdictCreator,
}

async fn simple_message<'a>(title: &'a str, message: &'a str) -> impl IntoResponse {
    Html(
        MessageTemplate {
            debug: true,
            title,
            description: None,
            image: None,
            player: None,
            header: title,
            message,
        }
        .render()
        .unwrap(),
    )
}

fn remove_all_html(content: &str) -> Cow<'_, str> {
    REMOVE_HTML_REGEX.replace_all(content, "")
}

async fn archive(
    extract::State(state): extract::State<Arc<State>>,
    maybe_ymd: Option<extract::Path<(i32, u32, u32)>>,
) -> impl IntoResponse {
    let date = if let Some(extract::Path((year, month, day))) = maybe_ymd {
        NaiveDate::from_ymd_opt(year, month, day).unwrap()
    } else {
        let maybe_dt = sqlx::query_scalar!("SELECT max(published) FROM nhkeasier_story")
            .fetch_one(&state.pool)
            .await
            .unwrap();
        NaiveDateTime::parse_from_str(maybe_dt.as_ref().unwrap(), "%Y-%m-%d %H:%M:%S")
            .unwrap()
            .date()
    };

    let rows = sqlx::query("SELECT * FROM nhkeasier_story WHERE date(published) = $1")
        .bind(date)
        .fetch_all(&state.pool)
        .await
        .unwrap();
    let stories: Vec<Story> = rows
        .iter()
        .map(|row| Story::from_row(row).unwrap())
        .collect();

    // find previous and next days with stories
    let previous_day = sqlx::query_scalar!(
        "
            SELECT published
            FROM nhkeasier_story
            WHERE date(published) < $1
            ORDER BY published DESC
            LIMIT 1
        ",
        date,
    )
    .fetch_optional(&state.pool)
    .await
    .unwrap()
    .map(|dt| dt.date());
    let next_day = sqlx::query_scalar!(
        "
            SELECT published
            FROM nhkeasier_story
            WHERE date(published) > $1
            ORDER BY published ASC
            LIMIT 1
        ",
        date,
    )
    .fetch_optional(&state.pool)
    .await
    .unwrap()
    .map(|dt| dt.date());

    let story = stories
        .iter()
        .find(|story| story.video_reencoded.is_some())
        .or_else(|| stories.iter().find(|story| story.image.is_some()))
        .unwrap_or_else(|| &stories[0]);

    let titles = stories
        .iter()
        .map(|story| story.title)
        .collect::<Vec<_>>()
        .join("\n");
    let contents = stories
        .iter()
        .flat_map(|story| story.content)
        .collect::<Vec<_>>()
        .join("\n");
    let content = titles + &contents;
    let edict = state.sub_edict_creator.from(&content).join("\n");
    let enamdict = state.sub_enamdict_creator.from(&content).join("\n");

    Html(
        ArchiveTemplate {
            debug: true,
            title: "Easier Japanese Practice",
            description: story
                .content
                .map(|content| remove_all_html(content))
                .as_deref(),
            image: story.image,
            player: None, // TODO
            header: &format!("Stories on {}", date.format("%Y-%m-%d")),
            previous_day,
            date,
            next_day,
            edict: Some(&edict),
            enamdict: Some(&enamdict),
            stories,
        }
        .render()
        .unwrap(),
    )
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

    // find ids of previous and next stories
    // dt = story.published.strftime('%Y-%m-%d %H:%M:%S')
    let dt = story.published;
    let previous_story_id = sqlx::query_scalar!(
        "
            SELECT id
            FROM nhkeasier_story
            WHERE (published, id) < ($1, $2)
            ORDER BY published DESC, id DESC
            LIMIT 1
        ",
        dt,
        id,
    )
    .fetch_optional(&state.pool)
    .await
    .unwrap();
    let next_story_id = sqlx::query_scalar!(
        "
            SELECT id
            FROM nhkeasier_story
            WHERE (published, id) > ($1, $2)
            ORDER BY published ASC, id ASC
            LIMIT 1
        ",
        dt,
        id,
    )
    .fetch_optional(&state.pool)
    .await
    .unwrap();

    let edict = story
        .content
        .as_ref()
        .map(|content| state.sub_edict_creator.from(content).join("\n"));
    let enamdict = story
        .content
        .as_ref()
        .map(|content| state.sub_enamdict_creator.from(content).join("\n"));

    Html(
        StoryTemplate {
            debug: true,
            title: story.title,
            description: story
                .content
                .map(|content| remove_all_html(content))
                .as_deref(),
            image: story.image,
            player: None,
            header: "Single Story",
            previous_story_id,
            next_story_id,
            edict: edict.as_deref(),
            enamdict: enamdict.as_deref(),
            story: &story,
        }
        .render()
        .unwrap(),
    )
}

async fn about() -> impl IntoResponse {
    Html(
        AboutTemplate {
            debug: true,
            title: "About",
            description: None,
            image: None,
            player: None,
            header: "About",
        }
        .render()
        .unwrap(),
    )
}

async fn contact() -> impl IntoResponse {
    Html(
        ContactTemplate {
            debug: true,
            title: "Contact",
            description: None,
            image: None,
            player: None,
            header: "Contact",
        }
        .render()
        .unwrap(),
    )
}

async fn contact_sent() -> impl IntoResponse {
    simple_message(
        "Message Sent",
        "Thank you for your feedback. We will take your message under \
        consideration as soon as possible.",
    )
    .await
}

async fn feed(extract::State(state): extract::State<Arc<State>>) -> impl IntoResponse {
    let rows =
        sqlx::query("SELECT * FROM nhkeasier_story ORDER BY published DESC, id DESC LIMIT 50")
            .fetch_all(&state.pool)
            .await
            .unwrap();
    let stories: Vec<Story> = rows
        .iter()
        .map(|row| Story::from_row(row).unwrap())
        .collect();
    let content = FeedTemplate { stories }.render().unwrap();
    ([(header::CONTENT_TYPE, "application/rss+xml")], content)
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
        .route("/", get(archive))
        .route("/:year/:month/:day/", get(archive))
        .route("/story/:id/", get(story))
        .route("/about/", get(about))
        .route("/contact/", get(contact))
        .route("/contact/sent/", get(contact_sent))
        .route("/feed/", get(feed))
        .nest_service("/media", ServeDir::new("../media"))
        .nest_service("/static", ServeDir::new("static"))
        .with_state(state);

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
