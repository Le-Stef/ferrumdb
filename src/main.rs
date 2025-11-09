use ferrumdb::{server, web, cluster::ClusterManager};
use tracing::{info, error};
use tracing_subscriber;
use std::sync::Arc;

// taskkill /F /IM ferrumdb.exe

#[tokio::main]
async fn main() {
    // Initialize logging (DEBUG level for detailed command tracing)
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::DEBUG.into())
        )
        .init();

    info!("FerrumDB starting...");

    // Server configuration
    let redis_addr = "127.0.0.1:6379";
    let web_addr = "127.0.0.1:8080";

    // Determine number of shards (one per CPU core, min 1, max 16)
    let num_cpus = num_cpus::get();
    let num_shards = num_cpus.clamp(1, 16);
    info!("Detected {} CPU cores, creating {} shards", num_cpus, num_shards);

    // Create cluster manager with AOF enabled
    let cluster = match ClusterManager::new(num_shards, true) {
        Ok(c) => Arc::new(c),
        Err(e) => {
            error!("Failed to initialize cluster: {}", e);
            std::process::exit(1);
        }
    };

    // Clone cluster for web server
    let web_cluster = cluster.clone();

    // Start RESP server in background task
    let redis_handle = tokio::spawn(async move {
        info!("Starting RESP server on {}", redis_addr);
        if let Err(e) = server::run_with_cluster(redis_addr, cluster).await {
            error!("RESP server error: {}", e);
        }
    });

    // Start Web server in background task
    let web_handle = tokio::spawn(async move {
        info!("Starting Web server on {}", web_addr);
        if let Err(e) = web::run_web_with_cluster(web_addr, web_cluster).await {
            error!("Web server error: {}", e);
        }
    });

    // Wait for both servers
    tokio::select! {
        _ = redis_handle => error!("RESP server stopped"),
        _ = web_handle => error!("Web server stopped"),
    }
}
