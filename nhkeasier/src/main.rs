use std::net::SocketAddr;
use std::process::exit;

use clap::Parser;
use futures::future::FutureExt;
use sqlx::{Pool, Sqlite};
use std::panic::AssertUnwindSafe;
use tokio::time::Duration;

const UPDATE_PERIOD: Duration = Duration::from_secs(3600);

async fn update_job(pool: Pool<Sqlite>) {
    loop {
        // Catch panics to avoid the situation where the server keeps running but the update job is
        // dead; the alternative would be to kill the whole process to make sure the panic is visible.
        if AssertUnwindSafe(nhkeasier::update_stories(&pool))
            .catch_unwind()
            .await
            .is_err()
        {
            tracing::error!("panic caught when running update_stories()");
        }
        tokio::time::sleep(UPDATE_PERIOD).await;
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
        eprintln!(".env file is missing");
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
    axum::serve(listener, app)
        .await
        .expect("failed to start server");
    Ok(())
}
