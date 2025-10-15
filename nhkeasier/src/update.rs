use std::borrow::Cow;
use std::collections::HashMap;
use std::process::Command;
use std::sync::LazyLock;

use chrono::NaiveDate;
use regex::Regex;
use reqwest::Client;
use serde::Deserialize;
use sqlx::FromRow;
use tracing::Instrument;

use crate::Story;

const BUILD_AUTHORIZE_URL: &str = "https://news.web.nhk/tix/build_authorize?idp=a-alaz&profileType=abroad&redirect_uri=https%3A%2F%2Fnews.web.nhk%2Fnews%2Feasy%2F&entity=none&area=130&pref=13&jisx0402=13101&postal=1000001";
const STORY_LIST_URL: &str = "https://news.web.nhk/news/easy/news-list.json";

static CONTENT_SELECTOR: LazyLock<scraper::Selector> =
    LazyLock::new(|| scraper::Selector::parse(".article-body").expect("invalid selector"));
static CLEAN_UP_CONTENT_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new("<a.*?>|<span.*?>|</a>|</span>|<p></p>").expect("invalid CLEAN_UP_CONTENT_REGEX")
});

#[derive(Clone, Debug, Deserialize)]
struct StoryInfo<'a> {
    // news_id and news_prearranged_time have a predefined format which should never contain
    // escapes, so we can always zero-copy parse them
    pub news_id: &'a str,
    pub news_prearranged_time: &'a str,
    // for some reason, they escape the slashes, so URLs can never be zero-copy parsed
    // TODO: https://github.com/dtolnay/request-for-implementation/issues/7
    pub news_web_image_uri: String,
    pub news_easy_image_uri: String,
    pub news_easy_voice_uri: String,
    // there might be escapes in the titles, but we might get away with zero-copy in some cases
    #[serde(borrow)]
    pub title: Cow<'a, str>,
    #[serde(borrow)]
    pub title_with_ruby: Cow<'a, str>,
}

#[derive(Clone, Debug, Deserialize)]
struct NewsList<'a>(#[serde(borrow)] [HashMap<NaiveDate, Vec<StoryInfo<'a>>>; 1]);

#[derive(Clone, Debug, Deserialize)]
struct VoiceToken {
    token: String,
}

use sqlx::{Pool, Sqlite, sqlite::SqliteRow};

async fn upsert_story(pool: &Pool<Sqlite>, info: &StoryInfo<'_>) -> (bool, SqliteRow) {
    let mut rows = sqlx::query("SELECT * FROM nhkeasier_story WHERE news_id = $1")
        .bind(info.news_id)
        .fetch_all(pool)
        .await
        .expect("failed to query database for existing story");
    // TODO: make news_id UNIQUE and use ON CONFLICT
    if let Some(row) = rows.pop() {
        assert!(rows.is_empty());
        (false, row)
    } else {
        let published = crate::parse_datetime_nhk(info.news_prearranged_time);
        (true, sqlx::query("INSERT INTO nhkeasier_story (news_id, published, title_with_ruby, title) VALUES ($1, $2, $3, $4) RETURNING *")
            .bind(info.news_id)
            .bind(published.naive_utc())
            .bind(info.title_with_ruby.as_ref())
            .bind(info.title.as_ref())
            .fetch_one(pool)
            .await
            .expect("failed to create new story")
        )
    }
}

async fn html_of_story(pool: &Pool<Sqlite>, client: &Client, story: &Story<'_>) -> String {
    if let Some(webpage) = story.webpage {
        tracing::debug!("getting HTML from database");
        // TODO: use Tokio fs
        std::fs::read_to_string(format!("media/{webpage}")).expect("failed to read existing HTML")
    } else {
        let url = format!(
            "http://www3.nhk.or.jp/news/easy/{0}/{0}.html",
            story.news_id
        );
        tracing::debug!("downloading HTML from {url}");
        let res = client
            .get(url)
            .send()
            .await
            .expect("failed to download HTML");
        let html = res.bytes().await.expect("failed to get HTML contents");
        tracing::debug!("saving HTML to file");
        let mut c = std::io::Cursor::new(&html);
        let filename = format!("html/{}.html", story.news_id);
        // TODO: use Tokio fs
        let mut f = std::fs::File::create(format!("media/{filename}"))
            .expect("failed to create file to save HTML");
        std::io::copy(&mut c, &mut f).expect("failed to save HTML");
        tracing::debug!("saving HTML to database");
        // TODO: no need to wait for query to finish
        sqlx::query!(
            "UPDATE nhkeasier_story SET webpage = $1 WHERE id = $2",
            filename,
            story.id
        )
        .execute(pool)
        .await
        .expect("failed to update webpage (story was removed from database while updating it)");
        tracing::debug!("decoding UTF-8 HTML");
        String::from_utf8(html.into()).expect("failed to decode HTML")
    }
}

fn raw_content_of_html(html: &str) -> Result<String, ()> {
    let document = scraper::Html::parse_document(html);
    if let Some(fragment) = document.select(&CONTENT_SELECTOR).next() {
        Ok(fragment.inner_html())
    } else {
        Err(())
    }
}

fn clean_up_html(content: &str) -> Cow<'_, str> {
    CLEAN_UP_CONTENT_REGEX.replace_all(content, "")
}

pub async fn extract_story_content(
    pool: &Pool<Sqlite>,
    client: &Client,
    story: &Story<'_>,
) -> Result<(), ()> {
    if story.content_with_ruby.is_some() {
        tracing::debug!("content already present");
        return Ok(());
    }
    let html = html_of_story(pool, client, story).await;
    tracing::debug!("extracting content");
    let raw_content = raw_content_of_html(&html)?;
    let content_with_ruby = clean_up_html(&raw_content);
    let content_with_ruby = content_with_ruby.trim();
    let content = crate::remove_ruby(content_with_ruby);
    tracing::debug!("saving content to database");
    // TODO: no need to wait for query to finish
    sqlx::query!(
        "UPDATE nhkeasier_story SET content_with_ruby = $1, content = $2 WHERE id = $3",
        content_with_ruby,
        content,
        story.id
    )
    .execute(pool)
    .await
    .expect("failed to update content (story was removed from database while updating it)");
    tracing::debug!("saved content to database");
    tracing::info!("found content for story");
    Ok(())
}

async fn fetch_image_of_story(
    pool: &Pool<Sqlite>,
    client: &Client,
    info: &StoryInfo<'_>,
    story: &Story<'_>,
) {
    if story.image.is_some() {
        tracing::debug!("image already present");
        return;
    }
    let url = if !info.news_web_image_uri.is_empty() {
        info.news_web_image_uri.to_string()
    } else if !info.news_easy_image_uri.is_empty() {
        info.news_easy_image_uri.to_string()
    } else {
        format!(
            "http://www3.nhk.or.jp/news/easy/{}/{}",
            story.news_id, info.news_easy_image_uri
        )
    };
    tracing::debug!("downloading image from {url}");
    let req = client.get(url).send().await;
    let res = req.expect("failed to download image");
    if res.status() == 404 {
        tracing::info!("got 404 when downloading image");
        return;
    }
    let content = res.bytes().await.expect("failed to get image contents");
    tracing::debug!("saving image to file");
    let mut c = std::io::Cursor::new(&content);
    let filename = format!("jpg/{}.jpg", story.news_id);
    let path = format!("media/{filename}");
    // TODO: use Tokio fs
    let mut f = std::fs::File::create(&path).expect("failed to create file to save image");
    std::io::copy(&mut c, &mut f).expect("failed to save image");
    tracing::debug!("making image progressive: mogrify -interlace plane {path}");
    let output = Command::new("mogrify")
        .args(["-interlace", "plane", &path])
        .output()
        .expect("failed to call mogrify -interlace plane …");
    if !output.status.success() {
        panic!("failed to make image progressive: {output:?}");
    }
    tracing::debug!("saving image to database");
    // TODO: no need to wait for query to finish
    sqlx::query!(
        "UPDATE nhkeasier_story SET image = $1 WHERE id = $2",
        filename,
        story.id
    )
    .execute(pool)
    .await
    .expect("failed to update image (story was removed from database while updating it)");
    tracing::debug!("saved image to database");
    tracing::info!("found image for story");
}

async fn get_voice_token(client: &Client) -> String {
    let res = client
        .get("https://mediatoken.web.nhk/v1/token")
        .send()
        .await
        .expect("failed to get voice token");
    let voice_token: VoiceToken = res
        .json()
        .await
        .expect("failed to parse voice token response");
    voice_token.token
}

async fn fetch_voice_of_story(
    pool: &Pool<Sqlite>,
    token: &str,
    info: &StoryInfo<'_>,
    story: &Story<'_>,
) {
    if story.voice.is_some() {
        tracing::debug!("voice already present");
        return;
    }
    tracing::debug!("downloading voice to file {}", info.news_easy_voice_uri);
    let voiceid = info
        .news_easy_voice_uri
        .strip_suffix(".m4a")
        .expect("failed to strip suffix .m4u");
    let url = format!("https://media.vd.st.nhk/news/easy_audio/{voiceid}/index.m3u8?hdnts={token}");
    let filename = format!("mp3/{}.mp3", story.news_id);
    let path = format!("media/{filename}");
    tracing::debug!(
        "running: vlc -I dummy {url} :sout=#transcode{{acodec=mp3,ab=192}}:std{{dst={path},access=file}} vlc://quit"
    );
    let output = Command::new("vlc")
        .args([
            "-I",
            "dummy",
            &url,
            &(String::from(":sout=#transcode{acodec=mp3,ab=192}:std{dst=")
                + &path
                + ",access=file}"),
            "vlc://quit",
        ])
        .output()
        .expect("failed to call vlc -I dummy …");
    if !output.status.success() {
        panic!("failed to download voice: {output:?}");
    }
    tracing::debug!("saving voice to database");
    // TODO: no need to wait for query to finish
    sqlx::query!(
        "UPDATE nhkeasier_story SET voice = $1 WHERE id = $2",
        filename,
        story.id
    )
    .execute(pool)
    .await
    .expect("failed to update voice (story was removed from database while updating it)");
    tracing::debug!("saved voice to database");
    tracing::info!("found voice for story");
}

async fn authenticate(client: &Client) {
    tracing::debug!("requesting build-authorize URL: {BUILD_AUTHORIZE_URL}");
    // NOTE: reqwest automatically follows redirections
    let res = client
        .get(BUILD_AUTHORIZE_URL)
        .send()
        .await
        .expect("failed to request build-authorize URL");
    if res.status() != 200 {
        tracing::error!("received error when getting JWT: {res:?}");
        tracing::error!("{}", res.text().await.unwrap());
    }
}

pub async fn update_stories(pool: &Pool<Sqlite>) {
    tracing::info!("Updating stories");
    tracing::debug!("downloading list of stories");
    let client = Client::builder()
        .cookie_store(true)
        .build()
        .expect("failed to build HTTP client");
    authenticate(&client).await;
    let res = client
        .get(STORY_LIST_URL)
        .send()
        .await
        .expect("failed to download list of stories");
    if res.status() != 200 {
        tracing::error!("received error when downloading list of stories: {res:?}");
        tracing::error!("{}", res.text().await.unwrap());
        return;
    }
    let data = res
        .text()
        .await
        .expect("failed to get contents of list of stories");
    tracing::debug!("downloaded list of stories");
    let j: NewsList = serde_json::from_str(&data).expect("failed to parse list of stories");
    let voice_token = get_voice_token(&client).await;
    for stories in j.0[0].values() {
        for info in stories {
            let span = tracing::debug_span!("story", news_id = info.news_id);
            let client = &client;
            let voice_token = &voice_token;
            async move {
                tracing::debug!("info={:?}", info);
                tracing::debug!("searching story for news_id={}", info.news_id);
                let (created, row) = upsert_story(pool, info).await;
                let story = Story::from_row(&row).expect("failed to convert row into Story");
                if created {
                    tracing::info!("new story id={} for news_id={}", story.id, story.news_id);
                } else {
                    tracing::debug!("selected id={} for news_id={}", story.id, story.news_id);
                }
                extract_story_content(pool, client, &story)
                    .await
                    .expect("failed to extract content");
                fetch_image_of_story(pool, client, info, &story).await;
                fetch_voice_of_story(pool, voice_token, info, &story).await;
            }
            .instrument(span)
            .await
        }
    }
}
