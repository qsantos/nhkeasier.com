use std::borrow::Cow;
use std::collections::HashMap;
use std::process::Command;

use chrono::NaiveDate;
use lazy_static::lazy_static;
use regex::Regex;
use serde::Deserialize;
use sqlx::FromRow;

use crate::Story;

const STORY_LIST_URL: &str = "http://www3.nhk.or.jp/news/easy/news-list.json";

lazy_static! {
    static ref STORY_CONTENT_REGEX: Regex = Regex::new(r#"(?s)<div class="article-main__body article-body" id="js-article-body">(.*?)            </div>"#).unwrap();
    static ref CLEAN_UP_CONTENT_REGEX: Regex = Regex::new("<a.*?>|<span.*?>|</a>|<span>|<p></p>").unwrap();
}

#[derive(Clone, Debug, Deserialize)]
struct StoryInfo<'a> {
    // news_id and news_prearranged_time have a predefined format which should never contain
    // escapes, so we can always zero-copy parse them
    pub news_id: &'a str,
    pub news_prearranged_time: &'a str,
    // for some reason, they escape the slashes, so URLs can never be zero-copy parsed
    // TODO: https://github.com/dtolnay/request-for-implementation/issues/7
    pub news_web_image_uri: String,
    // this is actually not a full URLs but just a filename, so no need to unescape
    pub news_easy_voice_uri: &'a str,
    // there might be escapes in the titles, but we might get away with zero-copy in some cases
    #[serde(borrow)]
    pub title: Cow<'a, str>,
    #[serde(borrow)]
    pub title_with_ruby: Cow<'a, str>,
}

#[derive(Clone, Debug, Deserialize)]
struct NewsList<'a>(#[serde(borrow)] [HashMap<NaiveDate, Vec<StoryInfo<'a>>>; 1]);

use sqlx::{sqlite::SqliteRow, Pool, Sqlite};

async fn upsert_story(pool: &Pool<Sqlite>, info: &StoryInfo<'_>) -> (bool, SqliteRow) {
    let mut rows = sqlx::query("SELECT * FROM nhkeasier_story WHERE story_id = $1")
        .bind(info.news_id)
        .fetch_all(pool)
        .await
        .unwrap();
    // TODO: make story_id UNIQUE and use ON CONFLICT
    if rows.is_empty() {
        let published = crate::parse_datetime_nhk(info.news_prearranged_time);
        (true, sqlx::query("INSERT INTO nhkeasier_story (story_id, published, title, title_with_ruby, subedict_created) VALUES ($1, $2, $3, $4, 0) RETURNING *")
            .bind(info.news_id)
            .bind(published.naive_utc())
            .bind(info.title_with_ruby.as_ref())
            .bind(info.title.as_ref())
            .fetch_one(pool)
            .await
            .unwrap())
    } else {
        assert_eq!(rows.len(), 1);
        (false, rows.pop().unwrap())
    }
}

async fn html_of_story(pool: &Pool<Sqlite>, story: &Story<'_>) -> String {
    if let Some(webpage) = story.webpage {
        tracing::debug!("getting HTML from database");
        // TODO: use Tokio fs
        std::fs::read_to_string(format!("media/{webpage}")).unwrap()
    } else {
        tracing::debug!("downloading HTML");
        let url = format!(
            "http://www3.nhk.or.jp/news/easy/{0}/{0}.html",
            story.story_id
        );
        let res = reqwest::get(url).await.unwrap();
        let html = res.bytes().await.unwrap();
        tracing::debug!("saving HTML to file");
        let mut c = std::io::Cursor::new(&html);
        let filename = format!("html/{}.html", story.story_id);
        // TODO: use Tokio fs
        let mut f = std::fs::File::create(format!("media/{filename}")).unwrap();
        std::io::copy(&mut c, &mut f).unwrap();
        tracing::debug!("saving HTML to database");
        // TODO: no need to wait for query to finish
        sqlx::query!(
            "UPDATE nhkeasier_story SET webpage = $1 WHERE id = $2",
            filename,
            story.id
        )
        .execute(pool)
        .await
        .unwrap();
        tracing::debug!("decoding UTF-8 HTML");
        String::from_utf8(html.into()).unwrap()
    }
}

async fn extract_story_content(pool: &Pool<Sqlite>, story: &Story<'_>, html: &str) {
    if story.content_with_ruby.is_some() {
        tracing::debug!("content already present");
        return;
    }
    tracing::debug!("extracting content");
    let captures = STORY_CONTENT_REGEX.captures(html).unwrap();
    let content_with_ruby = CLEAN_UP_CONTENT_REGEX.replace(&captures[1], "");
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
    .unwrap();
    tracing::debug!("saved content to database");
}

async fn fetch_image_of_story(pool: &Pool<Sqlite>, info: &StoryInfo<'_>, story: &Story<'_>) {
    if story.image.is_some() {
        tracing::debug!("image already present");
        return;
    }
    tracing::debug!("downloading image");
    let res = reqwest::get(&info.news_web_image_uri).await.unwrap();
    let content = res.bytes().await.unwrap();
    tracing::debug!("saving image to file");
    let mut c = std::io::Cursor::new(&content);
    let filename = format!("jpg/{}.jpg", story.story_id);
    let path = format!("media/{filename}");
    // TODO: use Tokio fs
    let mut f = std::fs::File::create(&path).unwrap();
    std::io::copy(&mut c, &mut f).unwrap();
    tracing::debug!("making image progressive");
    assert!(Command::new("mogrify")
        .args(["-interlace", "plane", &path])
        .output()
        .unwrap()
        .status
        .success());
    tracing::debug!("saving image to database");
    // TODO: no need to wait for query to finish
    sqlx::query!(
        "UPDATE nhkeasier_story SET image = $1 WHERE id = $2",
        filename,
        story.id
    )
    .execute(pool)
    .await
    .unwrap();
    tracing::debug!("saved image to database");
}

async fn fetch_voice_of_story(pool: &Pool<Sqlite>, info: &StoryInfo<'_>, story: &Story<'_>) {
    if story.voice.is_some() {
        tracing::debug!("voice already present");
        return;
    }
    tracing::debug!("downloading voice to file");
    let voiceid = info.news_easy_voice_uri.strip_suffix(".m4a").unwrap();
    let url = format!("https://vod-stream.nhk.jp/news/easy_audio/{voiceid}/index.m3u8");
    let filename = format!("mp3/{}.mp3", story.story_id);
    let path = format!("media/{filename}");
    assert!(Command::new("vlc")
        .args([
            "-I",
            "dummy",
            &url,
            &(String::from(":sout=#transcode{acodec=mpga,ab=192}:std{dst=")
                + &path
                + ",access=file}"),
            "vlc://quit",
        ])
        .output()
        .unwrap()
        .status
        .success());
    tracing::debug!("saving voice to database");
    // TODO: no need to wait for query to finish
    sqlx::query!(
        "UPDATE nhkeasier_story SET voice = $1 WHERE id = $2",
        filename,
        story.id
    )
    .execute(pool)
    .await
    .unwrap();
    tracing::debug!("saved voice to database");
}

pub async fn update_stories(pool: &Pool<Sqlite>) {
    tracing::info!("Updating stories");
    tracing::debug!("downloading list of stories");
    let res = reqwest::get(STORY_LIST_URL).await.unwrap();
    let data = res.text().await.unwrap();
    tracing::debug!("downloaded list of stories");
    let j: NewsList = serde_json::from_str(&data).unwrap();
    for stories in j.0[0].values() {
        for info in stories {
            tracing::debug!("searching story for news_id={}", info.news_id);
            let (created, row) = upsert_story(pool, info).await;
            let story = Story::from_row(&row).unwrap();
            if created {
                tracing::debug!("inserted id={} for story_id={}", story.id, story.story_id);
            } else {
                tracing::debug!("selected id={} for story_id={}", story.id, story.story_id);
            }
            let html = html_of_story(pool, &story).await;
            extract_story_content(pool, &story, &html).await;
            fetch_image_of_story(pool, info, &story).await;
            fetch_voice_of_story(pool, info, &story).await;
        }
    }
}
