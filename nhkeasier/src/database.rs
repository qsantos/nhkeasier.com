use std::path::Path;
use std::str::FromStr;
use std::time::Duration;

use chrono::NaiveDateTime;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{ConnectOptions, FromRow, Pool, Sqlite};

#[derive(Clone, Debug, FromRow)]
#[allow(dead_code)]
pub struct Story<'a> {
    pub id: i64,
    pub news_id: &'a str,
    pub published: NaiveDateTime,
    pub title_with_ruby: &'a str,
    pub title: &'a str,
    pub content_with_ruby: Option<&'a str>,
    pub content: Option<&'a str>,
    pub image: Option<&'a str>,
    pub voice: Option<&'a str>,
    pub webpage: Option<&'a str>,
}

impl Story<'_> {
    /// The path of a WebP variant of the image, relative to media/ (see update::image_variant)
    pub fn image_variant(&self, width: u32) -> String {
        crate::update::image_variant(self.image.unwrap_or_default(), width)
    }

    /// Whether the WebP variants of the image exist; otherwise, browsers would not fall back to
    /// the original image
    pub fn has_image_variants(&self) -> bool {
        crate::update::IMAGE_VARIANT_WIDTHS
            .iter()
            .all(|&width| Path::new(&format!("media/{}", self.image_variant(width))).exists())
    }
}

pub async fn connect_to_database() -> Pool<Sqlite> {
    let url = std::env::var("DATABASE_URL").expect("missing environment variable DATABASE_URL");
    let opts = SqliteConnectOptions::from_str(&url)
        .expect("invalid DATABASE_URL")
        .log_slow_statements(tracing::log::LevelFilter::Warn, Duration::from_millis(100));
    SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(opts)
        .await
        .expect("failed to connect to database")
}
