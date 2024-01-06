use std::net::SocketAddr;
use std::process::exit;

use clap::Parser;
use sqlx::{Pool, Sqlite};
use tokio::time::{Duration, Instant};

const UPDATE_PERIOD: Duration = Duration::from_secs(3600);

async fn update_job(pool: Pool<Sqlite>) {
    nhkeasier::update_stories(&pool).await;
    let mut last_run = Instant::now();
    loop {
        let elapsed = last_run.elapsed();
        if elapsed >= UPDATE_PERIOD {
            nhkeasier::update_stories(&pool).await;
            last_run = Instant::now();
        } else {
            tokio::time::sleep_until(last_run + UPDATE_PERIOD).await;
        }
    }
}

async fn normalize_voices(pool: &Pool<Sqlite>) {
    use sqlx::FromRow;
    let rows = sqlx::query("SELECT * FROM nhkeasier_story WHERE voice GLOB 'mp3/*_*.mp3'")
        .fetch_all(pool)
        .await
        .unwrap();
    for row in rows {
        let story = nhkeasier::Story::from_row(&row).unwrap();
        let voice = story.voice.unwrap();
        let media = std::path::Path::new("media");
        let old = media.join(voice);
        let new_name = format!("mp3/{}.mp3", story.story_id);
        let new = media.join(&new_name);
        assert!(old.exists());
        if new.exists() {
            let oldc = std::fs::read(&old).unwrap();
            let newc = std::fs::read(&new).unwrap();
            if oldc != newc {
                tracing::warn!(
                    "{old:?} ({} bytes) != {new:?} ({} bytes) for story {}",
                    oldc.len(),
                    newc.len(),
                    story.id,
                );
            }
        } else {
            tracing::info!("renaming {old:?} to {new:?} for story {}", story.id);
            std::fs::rename(&old, &new).unwrap();
        }
        tracing::debug!("using {new:?} instead of {old:?} for story {}", story.id);
        sqlx::query("UPDATE nhkeasier_story SET voice = $1 WHERE id = $2")
            .bind(&new_name)
            .bind(story.id)
            .execute(pool)
            .await
            .unwrap();
    }
}

#[derive(Clone, Debug, Parser)]
struct Args {
    #[arg(short, long, default_value = "127.0.0.1:3000")]
    listen_addr: SocketAddr,
}

#[tokio::main]
async fn main() -> Result<(), edict2::Error> {
    let args = Args::parse();

    if dotenvy::dotenv().is_err() {
        tracing::error!(".env file is missing");
        exit(1);
    }

    nhkeasier::init_logging();

    tracing::info!("Connecting to database");
    let pool = nhkeasier::connect_to_database().await;

    normalize_voices(&pool).await;

    tokio::spawn(update_job(pool.clone()));

    tracing::info!("Loading EDICT2");
    let sub_edict_creator = edict2::SubEdictCreator::from_files()?;

    tracing::info!("Loading ENAMDICT");
    let sub_enamdict_creator = edict2::SubEnamdictCreator::from_files()?;

    tracing::info!("Preparing Web service");
    let state = nhkeasier::State {
        pool,
        sub_edict_creator,
        sub_enamdict_creator,
    };
    let app = nhkeasier::router(state);

    let listener = tokio::net::TcpListener::bind(args.listen_addr)
        .await
        .unwrap_or_else(|e| {
            tracing::error!("could not bind to {}: {}", args.listen_addr, e);
            exit(1);
        });
    tracing::info!("Listening on http://{:?}", args.listen_addr);
    axum::serve(listener, app).await.unwrap();
    Ok(())
}
