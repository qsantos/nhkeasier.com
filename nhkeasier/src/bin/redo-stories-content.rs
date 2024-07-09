use std::process::exit;

use sqlx::FromRow;
use tracing::Instrument;

use nhkeasier::Story;

#[tokio::main]
async fn main() {
    if dotenvy::dotenv().is_err() {
        tracing::error!(".env file is missing");
        exit(1);
    }

    nhkeasier::init_logging();

    tracing::info!("Connecting to database");
    let pool = nhkeasier::connect_to_database().await;

    tracing::debug!("Reprocessing contents of older stories");
    let rows = sqlx::query("SELECT * FROM nhkeasier_story")
        .fetch_all(&pool)
        .await
        .expect("failed to query database for all stories");
    for row in rows {
        let mut story = Story::from_row(&row).expect("failed to convert row into Story");
        let span = tracing::debug_span!("story", news_id = story.news_id);
        story.content = None;
        story.content_with_ruby = None;
        tracing::debug!("Trying to re-derive the content from the HTML");
        if nhkeasier::extract_story_content(&pool, &story)
            .instrument(span)
            .await
            .is_ok()
        {
            tracing::info!("Re-derived the content from the HTML");
        } else {
            tracing::debug!("Failed to re-derive the content from the HTML");
        }
    }
    tracing::debug!("processed contents of older stories");
}
