use std::fs::File;
use std::io::Write;

use askama::Template;
use chrono::{FixedOffset, NaiveDateTime, TimeZone, Utc};
use regex::Regex;
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::{FromRow, Pool, Sqlite};
use zip::{write::FileOptions, CompressionMethod, ZipWriter};

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

lazy_static::lazy_static! {
    static ref FIX_IMG_TAGS_REGEX: Regex = Regex::new("<(img .*?)/?>").unwrap();
}

impl<'a> Story<'a> {
    fn xml_content_with_ruby(&self) -> String {
        let parser = libxml::parser::Parser::default_html();
        let content = self.content_with_ruby.unwrap();
        let document = parser.parse_string(content.as_bytes()).unwrap();
        let html = document.get_root_element().unwrap();
        let mut body = html.get_first_child().unwrap();
        body.set_name("div").unwrap();
        document.node_to_string(&body)
    }
}

fn zip_bytes(zip: &mut ZipWriter<File>, filename: &str, bytes: &[u8]) {
    let options = FileOptions::default().compression_method(CompressionMethod::DEFLATE);
    zip.start_file(filename, options).unwrap();
    zip.write_all(bytes).unwrap();
}

fn zip_bytes_store(zip: &mut ZipWriter<File>, filename: &str, bytes: &[u8]) {
    let options = FileOptions::default().compression_method(CompressionMethod::STORE);
    zip.start_file(filename, options).unwrap();
    zip.write_all(bytes).unwrap();
}

fn zip_template<T: Template>(zip: &mut ZipWriter<File>, filename: &str, template: T) {
    let content = template.render().unwrap();
    zip_bytes(zip, filename, content.as_bytes());
}

macro_rules! zip_copy {
    ( $zip:expr, $filename:expr ) => {
        zip_bytes(
            $zip,
            $filename,
            include_bytes!(concat!("../templates/", $filename)),
        );
    };
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
    let mut zip = ZipWriter::new(f);

    zip_bytes(&mut zip, "mimetype", b"application/epub+zip");
    zip_copy!(&mut zip, "META-INF/container.xml");
    zip_copy!(&mut zip, "META-INF/com.apple.ibooks.display-options.xml");
    let template = ContentOpfTemplate { stories: &stories };
    zip_template(&mut zip, "EPUB/content.opf", template);
    let template = NavXhtmlTemplate { stories: &stories };
    zip_template(&mut zip, "EPUB/nav.xhtml", template);
    let template = TocNcxTemplate { stories: &stories };
    zip_template(&mut zip, "EPUB/toc.ncx", template);
    zip_copy!(&mut zip, "EPUB/styles/stylesheet.css");
    // zip_copy!(&mut zip, "EPUB/fonts/NotoSansCJKjp-VF.otf");
    zip_copy!(&mut zip, "EPUB/text/title_page.xhtml");
    for story in stories.iter() {
        let filename = format!("EPUB/text/{}.xhtml", story.story_id);
        zip_template(&mut zip, &filename, StoryTemplate { story });
    }
    zip_copy!(&mut zip, "EPUB/images/logo.png");
    for story in stories {
        if let Some(image) = story.image {
            if image.is_empty() {
                continue;
            }
            let data = std::fs::read(format!("media/{}", image)).unwrap();
            let filename = format!("EPUB/images/{}.jpg", story.story_id);
            zip_bytes_store(&mut zip, &filename, &data);
        }
    }

    zip.finish().unwrap();
}
