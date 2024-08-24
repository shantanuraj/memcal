use axum::{
    routing::{get, post},
    Router,
};
use dotenvy::dotenv;
use sqlx::sqlite::SqlitePool;
use std::net::SocketAddr;
use tracing::info;

mod api;
mod db;
mod ical;
mod web;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    dotenv().ok();

    let db_addr =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:data/memcal.db".to_string());

    let db_pool = SqlitePool::connect(&db_addr).await.unwrap();
    db::init_db(&db_pool).await.unwrap();

    let app = Router::new()
        .route("/", get(web::index))
        .route("/feed", post(api::add_feed))
        .route("/feed/:id", get(api::get_feed).delete(api::delete_feed))
        .with_state(db_pool.clone());

    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse::<u16>()
        .expect("PORT must be a number");

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

    info!("Listening on http://localhost:{}", port);

    // Spawn a background task to sync feeds every 5 minutes
    let sync_pool = db_pool.clone();
    tokio::spawn(async move {
        loop {
            match db::get_all_feeds(&sync_pool).await {
                Ok(feeds) => {
                    for feed in feeds {
                        if let Err(e) = ical::sync_ical_events(&sync_pool, feed.id, &feed.url).await
                        {
                            eprintln!("Error syncing feed {}: {}", feed.id, e);
                        } else {
                            info!("Synced feed {}", feed.id);
                        }
                    }
                }
                Err(e) => eprintln!("Error fetching feeds: {}", e),
            }
            tokio::time::sleep(std::time::Duration::from_secs(300)).await;
        }
    });

    axum::serve(listener, app.with_state(db_pool))
        .await
        .unwrap();
}
