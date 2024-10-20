use axum::{
    routing::{delete, get, post},
    Router,
};
use dotenvy::dotenv;
use sqlx::sqlite::SqlitePool;
use std::net::SocketAddr;
use tower_http::services::ServeDir;
use tracing::info;

mod api;
mod db;
mod ical;
mod logger;
mod web;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    dotenv().ok();

    let db_addr =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:data/memcal.db".to_string());

    ensure_db_exists(&db_addr);

    let db_pool = SqlitePool::connect(&db_addr).await.unwrap();
    db::init_db(&db_pool).await.unwrap();

    let app = Router::new()
        .route("/", get(web::index))
        .route("/feed", post(api::add_feed))
        .route("/feed/:id", get(api::get_feed))
        .route(
            "/feed/:id/:manage_token",
            get(web::feed_page)
                .delete(api::delete_feed)
                .post(api::delete_feed),
        )
        .route(
            "/feed/:id/:event_id/:manage_token",
            delete(api::delete_event).post(api::delete_event),
        )
        .route("/robots.txt", get(web::robots_txt))
        .nest_service("/public", ServeDir::new("public"))
        .with_state(db_pool.clone())
        .layer(axum::middleware::from_fn(logger::log_request_response));

    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse::<u16>()
        .expect("PORT must be a number");

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

    info!("Listening on http://localhost:{}", port);

    let sync_interval = std::env::var("SYNC_INTERVAL")
        .unwrap_or_else(|_| "300".to_string())
        .parse::<u64>()
        .expect("SYNC_INTERVAL must be a number");

    info!("Syncing feeds every {} seconds", sync_interval);

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
            tokio::time::sleep(std::time::Duration::from_secs(sync_interval)).await;
        }
    });

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
}

fn ensure_db_exists(db_addr: &str) {
    let path = db_addr.split(':').nth(1).unwrap();
    if !std::path::Path::new(path).exists() {
        std::fs::create_dir_all(std::path::Path::new(path).parent().unwrap())
            .expect("Failed to create database directory");
        std::fs::File::create(path).expect("Failed to intialize database file");
    }
}
