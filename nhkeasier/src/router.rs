use std::any::Any;
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::Arc;

use askama_axum::Template;
use axum::{
    body::Body,
    extract,
    http::{header, Response, StatusCode},
    response::{Html, IntoResponse, Redirect},
    routing::get,
    Router,
};
use chrono::{Duration, Local, NaiveDate, NaiveDateTime, TimeZone};
use lazy_static::lazy_static;
use regex::Regex;
use serde::Deserialize;
use sqlx::FromRow;
use tower_http::catch_panic::CatchPanicLayer;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;

use edict2::{SubEdictCreator, SubEnamdictCreator};

use crate::JST;

lazy_static! {
    static ref REMOVE_HTML_REGEX: Regex = Regex::new("<.*?>").expect("invalid REMOVE_HTML_REGEX");
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
#[template(path = "feed.rss", escape = "xml")]
struct FeedTemplate<'a> {
    stories: Vec<Story<'a>>,
    furiganas: bool,
}

fn simple_message<'a>(title: &'a str, message: &'a str) -> Html<String> {
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
        .expect("failed to render message.html template"),
    )
}

async fn handle_not_found() -> impl IntoResponse {
    (
        StatusCode::NOT_FOUND,
        simple_message(
            "Page Not Found",
            "Sorry, we could not find the page you requested. Maybe the URL \
            you followed is incomplete, or the document has been moved.",
        ),
    )
}

fn handle_panic(_err: Box<dyn Any + Send + 'static>) -> Response<Body> {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        simple_message(
            "Server Error",
            "Sorry, something went very wrong on the server and we were not \
            able to display the requested document.",
        ),
    )
        .into_response()
}

fn remove_all_html(content: &str) -> Cow<'_, str> {
    REMOVE_HTML_REGEX.replace_all(content, "")
}

async fn archive(
    extract::State(state): extract::State<Arc<State>>,
    maybe_ymd: Option<extract::Path<(i32, u32, u32)>>,
) -> impl IntoResponse {
    let date = if let Some(extract::Path((year, month, day))) = maybe_ymd {
        if let Some(date) = NaiveDate::from_ymd_opt(year, month, day) {
            date
        } else {
            return (
                StatusCode::BAD_REQUEST,
                simple_message(
                    "Bad request",
                    &format!(
                        "{year}-{month}-{day} is not a valid date. \
                        You might have stumbled there my mistyping the URL; please double-check!"
                    ),
                ),
            );
        }
    } else {
        let maybe_dt = sqlx::query_scalar!("SELECT max(published) FROM nhkeasier_story")
            .fetch_one(&state.pool)
            .await
            .expect("failed to query database for max(published)");
        let dt = maybe_dt.as_ref().expect("database is empty");
        NaiveDateTime::parse_from_str(dt, "%Y-%m-%d %H:%M:%S")
            .expect("failed to parse published column from database")
            .date()
    };
    let tomorrow = date + Duration::days(1);

    let rows =
        sqlx::query("SELECT * FROM nhkeasier_story WHERE $1 <= published AND published < $2")
            .bind(date)
            .bind(tomorrow)
            .fetch_all(&state.pool)
            .await
            .expect("failed to query database for day stories");
    let stories: Vec<Story> = rows
        .iter()
        .map(|row| Story::from_row(row).expect("failed to convert row into Story"))
        .collect();

    // find previous and next days with stories
    let previous_day = sqlx::query_scalar!(
        "
            SELECT published
            FROM nhkeasier_story
            WHERE published < $1
            ORDER BY published DESC
            LIMIT 1
        ",
        date,
    )
    .fetch_optional(&state.pool)
    .await
    .expect("failed to query database for previous day")
    .map(|dt| dt.date());
    let next_day = sqlx::query_scalar!(
        "
            SELECT published
            FROM nhkeasier_story
            WHERE published >= $1
            ORDER BY published ASC
            LIMIT 1
        ",
        tomorrow,
    )
    .fetch_optional(&state.pool)
    .await
    .expect("failed to query database for next day")
    .map(|dt| dt.date());

    let story = stories
        .iter()
        .find(|story| story.video_reencoded.is_some())
        .or_else(|| stories.iter().find(|story| story.image.is_some()))
        .unwrap_or_else(|| stories.get(0).unwrap());

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

    (
        StatusCode::OK,
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
            .expect("failed to render index.html template"),
        ),
    )
}

async fn story(
    extract::State(state): extract::State<Arc<State>>,
    extract::Path(id): extract::Path<i64>,
) -> impl IntoResponse {
    let row = sqlx::query("SELECT * FROM nhkeasier_story WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.pool)
        .await
        .expect("failed to query database for specific story")
        .unwrap();
    let story = Story::from_row(&row).expect("failed to convert row to Story");

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
    .expect("failed to query database for previous story");
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
    .expect("failed to query database for next story");

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
        .expect("failed to render story.html template"),
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
        .expect("failed to render about.html template"),
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
        .expect("failed to render contact.html template"),
    )
}

#[derive(Clone, Debug, Deserialize)]
struct ContactForm {
    from_email: String,
    subject: String,
    message: String,
}

async fn contact_send(form: extract::Form<ContactForm>) -> impl IntoResponse {
    crate::send_email_async(
        &form.subject,
        format!("From: {}\n\n{}", form.from_email, form.message),
    )
    .await;
    Redirect::to("/contact/sent/")
}

async fn contact_sent() -> impl IntoResponse {
    simple_message(
        "Message Sent",
        "Thank you for your feedback. We will take your message under \
        consideration as soon as possible.",
    )
}

async fn feed(
    extract::State(state): extract::State<Arc<State>>,
    extract::Query(query): extract::Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let rows =
        sqlx::query("SELECT * FROM nhkeasier_story ORDER BY published DESC, id DESC LIMIT 50")
            .fetch_all(&state.pool)
            .await
            .expect("failed to query database for last stories");
    let stories: Vec<Story> = rows
        .iter()
        .map(|row| Story::from_row(row).expect("failed to convert row to Story"))
        .collect();
    let content = FeedTemplate {
        stories,
        furiganas: !query.contains_key("no-furiganas"),
    }
    .render()
    .expect("failed to render feed.rss template");
    ([(header::CONTENT_TYPE, "application/rss+xml")], content)
}

pub struct State {
    pub pool: sqlx::Pool<sqlx::Sqlite>,
    pub sub_edict_creator: SubEdictCreator,
    pub sub_enamdict_creator: SubEnamdictCreator,
}

pub fn router(state: State) -> Router {
    let middleware = tower::ServiceBuilder::new().layer(CatchPanicLayer::custom(handle_panic));

    let tracing = TraceLayer::new_for_http()
        .make_span_with(tower_http::trace::DefaultMakeSpan::new().level(tracing::Level::INFO))
        .on_response(tower_http::trace::DefaultOnResponse::new().level(tracing::Level::INFO));

    Router::new()
        .route("/", get(archive))
        .route("/:year/:month/:day/", get(archive))
        .route("/story/:id/", get(story))
        .route("/about/", get(about))
        .route("/contact/", get(contact).post(contact_send))
        .route("/contact/sent/", get(contact_sent))
        .route("/feed/", get(feed))
        .route_service("/robots.txt", ServeFile::new("nhkeasier/static/robots.txt"))
        .route_service(
            "/favicon.ico",
            ServeFile::new("nhkeasier/static/favicon.ico"),
        )
        .nest_service("/media", ServeDir::new("media"))
        .nest_service("/static", ServeDir::new("nhkeasier/static"))
        .fallback(handle_not_found)
        .layer(middleware)
        .layer(tracing)
        .with_state(Arc::new(state))
}
