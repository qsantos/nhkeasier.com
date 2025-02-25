use std::net::SocketAddr;
use std::process::exit;

use clap::Parser;
use futures::future::FutureExt;
use sqlx::{Pool, Sqlite};
use std::panic::AssertUnwindSafe;
use tokio::time::{Duration, Instant};

const UPDATE_PERIOD: Duration = Duration::from_secs(3600);

async fn update_job(pool: Pool<Sqlite>) {
    nhkeasier::update_stories(&pool).await;
    let mut last_run = Instant::now();
    loop {
        let elapsed = last_run.elapsed();
        if elapsed >= UPDATE_PERIOD {
            // Catch panics to avoid the situation where the server keeps running but the update job is
            // dead; the alternative would be to kill the whole process to make sure the panic is visible.
            if let Err(err) = AssertUnwindSafe(nhkeasier::update_stories(&pool))
                .catch_unwind()
                .await
            {
                tracing::error!("update job panicked: {:?}", err);
            }
            last_run = Instant::now();
        } else {
            tokio::time::sleep_until(last_run + UPDATE_PERIOD).await;
        }
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

    if !nhkeasier::DEBUG {
        tokio::spawn(update_job(pool.clone()));
    }

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
