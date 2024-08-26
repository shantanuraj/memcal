use std::net::SocketAddr;

use axum::{body::Body, extract::ConnectInfo};
use tracing::info;

pub async fn log_request_response(
    ConnectInfo(conn_info): ConnectInfo<SocketAddr>,
    req: axum::http::Request<axum::body::Body>,
    next: axum::middleware::Next,
) -> Result<axum::http::Response<Body>, (axum::http::StatusCode, String)> {
    let method = req.method().clone();
    let uri = req.uri().clone();
    let start = std::time::Instant::now();
    let header_map = &req.headers().clone();
    let socket_addr = &conn_info.to_string();
    let remote_addr = header_map
        .get("x-forwarded-for")
        .and_then(|x| x.to_str().ok())
        .unwrap_or(socket_addr.as_str());

    let response = next.run(req).await;

    let duration = start.elapsed();
    let status_code = response.status().as_u16();

    info!(
        "[axum] {} | {:?} | {} | {:?}\t{:?}",
        status_code, duration, remote_addr, method, uri
    );

    Ok(response)
}
