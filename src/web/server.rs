//! HTTP server implementation

use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::cors::CorsLayer;
use tracing::info;

use crate::dispatch::Dispatcher;
use crate::cluster::ClusterManager;
use super::handlers::{index_handler, execute_command, execute_command_cluster, stats_handler, stats_handler_cluster, shard_stats_handler};

/// Run the web server
pub async fn run_web_server(
    addr: &str,
    dispatcher: Arc<Mutex<Dispatcher>>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Build the application router
    let app = Router::new()
        .route("/", get(index_handler))
        .route("/command", post(execute_command))
        .route("/stats", get(stats_handler))
        .layer(CorsLayer::permissive())
        .with_state(dispatcher);

    // Start the server
    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!("Web interface available at http://{}", addr);

    axum::serve(listener, app).await?;

    Ok(())
}

/// Run the web server with cluster manager
pub async fn run_web_with_cluster(
    addr: &str,
    cluster: Arc<ClusterManager>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Build the application router
    let app = Router::new()
        .route("/", get(index_handler))
        .route("/command", post(execute_command_cluster))
        .route("/stats", get(stats_handler_cluster))
        .route("/shards", get(shard_stats_handler))
        .layer(CorsLayer::permissive())
        .with_state(cluster);

    // Start the server
    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!("Web interface available at http://{}", addr);

    axum::serve(listener, app).await?;

    Ok(())
}
