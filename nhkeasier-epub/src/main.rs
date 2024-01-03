use askama::Template;
use chrono::{FixedOffset, NaiveDateTime, TimeZone, Utc};
use regex::Regex;
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::{FromRow, Pool, Sqlite};

// TODO: deduplicate
#[derive(Clone, Debug, FromRow)]
#[allow(dead_code)]
pub struct Story<'a> {
    pub id: i64,
    pub story_id: &'a str,
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

// TODO: deduplicate
pub async fn connect_to_database() -> Pool<Sqlite> {
    let database_url =
        std::env::var("DATABASE_URL").expect("missing environment variable DATABASE_URL");
    SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("failed to connect to database")
}

// TODO: deduplicate
lazy_static::lazy_static! {
    pub static ref JST: FixedOffset = FixedOffset::east_opt(3600 * 9).unwrap();
}

#[derive(Template)]
#[template(path = "EPUB/content.opf", escape = "xml")]
struct ContentOpfTemplate<'a> {
    stories: &'a Vec<Story<'a>>,
}

#[derive(Template)]
#[template(path = "EPUB/nav.xhtml", escape = "xml")]
struct NavXhtmlTemplate<'a> {
    stories: &'a Vec<Story<'a>>,
}

#[derive(Template)]
#[template(path = "EPUB/toc.ncx", escape = "xml")]
struct TocNcxTemplate<'a> {
    stories: &'a Vec<Story<'a>>,
}

#[derive(Template)]
#[template(path = "EPUB/text/story.xhtml", escape = "xml")]
struct StoryTemplate<'a> {
    story: &'a Story<'a>,
}

use std::fs::File;
use std::io::Write;

lazy_static::lazy_static! {
    static ref FIX_IMG_TAGS_REGEX: Regex = Regex::new("<(img .*?)/?>").unwrap();
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().unwrap();

    let pool = connect_to_database().await;
    let rows = sqlx::query("SELECT * FROM nhkeasier_story ORDER BY published DESC LIMIT 20")
        .fetch_all(&pool)
        .await
        .unwrap();
    let stories: Vec<Story> = rows
        .iter()
        .map(|row| Story::from_row(row).expect("failed to convert row into Story"))
        .collect();

    // let mut buf = [0; 65536];
    let f = File::create("a.epub").unwrap();
    let mut zip = zip::ZipWriter::new(f);

    let options =
        zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    // mimetype
    zip.start_file("mimetype", options).unwrap();
    zip.write_all(b"application/epub+zip").unwrap();

    // META-INF/container.xml
    zip.start_file("META-INF/container.xml", options).unwrap();
    zip.write_all(include_bytes!("../templates/META-INF/container.xml"))
        .unwrap();

    // META-INF/com.apple.ibooks.display-options.xml
    zip.start_file("META-INF/com.apple.ibooks.display-options.xml", options)
        .unwrap();
    zip.write_all(include_bytes!(
        "../templates/META-INF/com.apple.ibooks.display-options.xml"
    ))
    .unwrap();

    // TODO
    // EPUB/content.opf
    zip.start_file("EPUB/content.opf", options).unwrap();
    let content = ContentOpfTemplate { stories: &stories }.render().unwrap();
    zip.write_all(content.as_bytes()).unwrap();

    // EPUB/nav.xhtml
    zip.start_file("EPUB/nav.xhtml", options).unwrap();
    let content = NavXhtmlTemplate { stories: &stories }.render().unwrap();
    zip.write_all(content.as_bytes()).unwrap();

    // EPUB/toc.ncx
    zip.start_file("EPUB/toc.ncx", options).unwrap();
    let content = TocNcxTemplate { stories: &stories }.render().unwrap();
    zip.write_all(content.as_bytes()).unwrap();

    // EPUB/styles/stylesheet.css
    zip.start_file("EPUB/styles/stylesheet.css", options)
        .unwrap();
    zip.write_all(include_bytes!("../templates/EPUB/styles/stylesheet.css"))
        .unwrap();

    // TODO
    if false {
        // EPUB/styles/NotoSansCJKjp-VF.css
        zip.start_file("EPUB/fonts/NotoSansCJKjp-VF.otf", options)
            .unwrap();
        zip.write_all(include_bytes!(
            "../templates/EPUB/fonts/NotoSansCJKjp-VF.otf"
        ))
        .unwrap();
    }

    // EPUB/text/title_page.xhtml
    zip.start_file("EPUB/text/title_page.xhtml", options)
        .unwrap();
    zip.write_all(include_bytes!("../templates/EPUB/text/title_page.xhtml"))
        .unwrap();

    // EPUB/text/k*.xhtml
    for story in stories.iter() {
        zip.start_file(format!("EPUB/text/{}.xhtml", story.story_id), options)
            .unwrap();
        let content = StoryTemplate { story }.render().unwrap();
        zip.write_all(content.as_bytes()).unwrap();
    }

    // EPUB/images/logo.png
    zip.start_file("EPUB/images/logo.png", options).unwrap();
    zip.write_all(include_bytes!("../templates/EPUB/images/logo.png"))
        .unwrap();

    // EPUB/images/k*.jpg
    for story in stories {
        if let Some(image) = story.image {
            if image.is_empty() {
                continue;
            }
            zip.start_file(format!("EPUB/images/{}.jpg", story.story_id), options)
                .unwrap();
            let data = std::fs::read(format!("media/{}", image)).unwrap();
            zip.write_all(&data).unwrap();
        }
    }

    zip.finish().unwrap();
}
