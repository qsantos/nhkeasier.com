use std::io::{Seek, Write};

use askama::Template;
use chrono::{DateTime, TimeZone, Utc};
use zip::{write::FileOptions, CompressionMethod, ZipWriter};

use crate::{Story, JST};

#[derive(Template)]
#[template(path = "epub/EPUB/content.opf", escape = "xml")]
struct ContentOpfTemplate<'a> {
    now: DateTime<Utc>,
    title: &'a str,
    stories: &'a [Story<'a>],
    with_images: bool,
    with_cjk_font: bool,
}

#[derive(Template)]
#[template(path = "epub/EPUB/nav.xhtml", escape = "xml")]
struct NavXhtmlTemplate<'a> {
    title: &'a str,
    stories: &'a [Story<'a>],
}

#[derive(Template)]
#[template(path = "epub/EPUB/toc.ncx", escape = "xml")]
struct TocNcxTemplate<'a> {
    title: &'a str,
    stories: &'a [Story<'a>],
}

#[derive(Template)]
#[template(path = "epub/EPUB/styles/stylesheet.css", escape = "none")]
struct StylesheetTemplate {
    with_cjk_font: bool,
}

#[derive(Template)]
#[template(path = "epub/EPUB/text/title_page.xhtml", escape = "xml")]
struct TitlePageTemplate<'a> {
    now: DateTime<Utc>,
    title: &'a str,
}

#[derive(Template)]
#[template(path = "epub/EPUB/text/story.xhtml", escape = "xml")]
struct StoryTemplate<'a> {
    story: &'a Story<'a>,
    with_furigana: bool,
    with_images: bool,
}

mod filters {
    pub fn xhtml_sanitize(content: &str) -> ::askama::Result<String> {
        let parser = libxml::parser::Parser::default_html();
        let document = parser.parse_string(content.as_bytes()).unwrap();
        let html = document.get_root_element().unwrap();
        let mut body = html.get_first_child().unwrap();
        body.set_name("div").unwrap();
        Ok(document.node_to_string(&body))
    }
}

fn zip_bytes<W: Write + Seek>(zip: &mut ZipWriter<W>, filename: &str, bytes: &[u8]) {
    let options = FileOptions::default().compression_method(CompressionMethod::DEFLATE);
    zip.start_file(filename, options).unwrap();
    zip.write_all(bytes).unwrap();
}

fn zip_bytes_store<W: Write + Seek>(zip: &mut ZipWriter<W>, filename: &str, bytes: &[u8]) {
    let options = FileOptions::default().compression_method(CompressionMethod::STORE);
    zip.start_file(filename, options).unwrap();
    zip.write_all(bytes).unwrap();
}

fn zip_template<W: Write + Seek, T: Template>(zip: &mut ZipWriter<W>, filename: &str, template: T) {
    let content = template.render().unwrap();
    zip_bytes(zip, filename, content.as_bytes());
}

macro_rules! zip_copy {
    ( $zip:expr, $filename:expr ) => {
        zip_bytes(
            $zip,
            $filename,
            include_bytes!(concat!("../templates/epub/", $filename)),
        );
    };
}

macro_rules! zip_copy_store {
    ( $zip:expr, $filename:expr ) => {
        zip_bytes_store(
            $zip,
            $filename,
            include_bytes!(concat!("../templates/epub/", $filename)),
        );
    };
}

pub fn make_epub<W: Write + Seek>(
    stories: &[Story<'_>],
    title: &str,
    output: W,
    with_furigana: bool,
    with_images: bool,
    with_cjk_font: bool,
) {
    let now = Utc::now();
    let mut zip = ZipWriter::new(output);

    zip_bytes(&mut zip, "mimetype", b"application/epub+zip");
    zip_copy!(&mut zip, "META-INF/container.xml");
    zip_copy!(&mut zip, "META-INF/com.apple.ibooks.display-options.xml");
    let template = ContentOpfTemplate {
        now,
        title,
        stories,
        with_images,
        with_cjk_font,
    };
    zip_template(&mut zip, "EPUB/content.opf", template);
    let template = NavXhtmlTemplate { title, stories };
    zip_template(&mut zip, "EPUB/nav.xhtml", template);
    let template = TocNcxTemplate { title, stories };
    zip_template(&mut zip, "EPUB/toc.ncx", template);
    let template = StylesheetTemplate { with_cjk_font };
    zip_template(&mut zip, "EPUB/styles/stylesheet.css", template);
    if with_cjk_font {
        zip_copy_store!(&mut zip, "EPUB/fonts/NotoSansCJKjp-VF.otf");
    }
    let template = TitlePageTemplate { now, title };
    zip_template(&mut zip, "EPUB/text/title_page.xhtml", template);
    for story in stories.iter() {
        let filename = format!("EPUB/text/{}.xhtml", story.story_id);
        zip_template(
            &mut zip,
            &filename,
            StoryTemplate {
                story,
                with_furigana,
                with_images,
            },
        );
    }
    zip_copy!(&mut zip, "EPUB/images/logo.png");
    if with_images {
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
    }

    zip.finish().unwrap();
}

#[tokio::test]
async fn test() {
    dotenvy::dotenv().unwrap();

    let pool = crate::connect_to_database().await;
    let rows = sqlx::query("SELECT * FROM nhkeasier_story ORDER BY published LIMIT 20")
        .fetch_all(&pool)
        .await
        .unwrap();
    use sqlx::FromRow;
    let stories: Vec<Story> = rows
        .iter()
        .map(|row| Story::from_row(row).expect("failed to convert row into Story"))
        .collect();

    //let mut buf = Vec::new();
    //let f = std::io::Cursor::new(&mut buf);
    let f = std::io::BufWriter::new(std::fs::File::create("a.epub").unwrap());
    make_epub(&stories, "NHK Easier latest stories", f, false, true, true);
}
