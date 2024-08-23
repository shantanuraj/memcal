use std::net::SocketAddr;
use axum::{
    routing::{get, post},
    Router,
};
use sqlx::sqlite::SqlitePool;
use tracing::info;

mod api;
mod db;
mod web;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let db_pool = SqlitePool::connect("sqlite:data/memcal.db").await.unwrap();
    db::init_db(&db_pool).await.unwrap();

    let app = Router::new()
        .route("/", get(web::index))
        .route("/feed", post(api::add_feed))
        .route("/feed/:id", get(api::get_feed).delete(api::delete_feed))
        .with_state(db_pool);


    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse::<u16>()
        .expect("PORT must be a number");

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

    info!("Listening on http://localhost:{}", port);

    axum::serve(listener, app).await.unwrap();
}
