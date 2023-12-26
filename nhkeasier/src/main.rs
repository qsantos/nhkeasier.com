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

#[tokio::main]
async fn main() -> Result<(), edict2::Error> {
    dotenvy::dotenv().unwrap();

    nhkeasier::init_logging();

    tracing::info!("Connecting to database");
    let pool = nhkeasier::connect_to_database().await;

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

    let listen: std::net::SocketAddr = "127.0.0.1:3000".parse().unwrap();
    tracing::info!("Listening on http://{listen:?}");

    let listener = tokio::net::TcpListener::bind(listen).await.unwrap();
    axum::serve(listener, app).await.unwrap();
    Ok(())
}
