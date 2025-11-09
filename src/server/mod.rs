//! Server module
//!
//! Handles TCP connections and manages the event loop.
//! This module is responsible for accepting connections and delegating
//! command processing to the dispatcher.

mod connection;

use crate::dispatch::Dispatcher;
use crate::cluster::ClusterManager;
use crate::aof::AofConfig;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tracing::{info, error};

pub use connection::Connection;

/// Run the server
///
/// Starts the TCP server on the given address and processes incoming connections.
pub async fn run(addr: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Create the dispatcher with AOF support
    // For Phase 1-2, we use a single dispatcher protected by a mutex.
    // In future phases, we'll implement sharding with multiple dispatchers.
    let aof_config = AofConfig::default();
    let dispatcher = Arc::new(Mutex::new(
        Dispatcher::with_aof(aof_config)
            .map_err(|e| format!("Failed to initialize AOF: {}", e))?
    ));

    run_with_dispatcher(addr, dispatcher).await
}

/// Run the server with a provided dispatcher
///
/// Allows sharing the same dispatcher between multiple servers (RESP and HTTP).
pub async fn run_with_dispatcher(
    addr: &str,
    dispatcher: Arc<Mutex<Dispatcher>>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Bind the TCP listener
    let listener = TcpListener::bind(addr).await?;
    info!("FerrumDB RESP server listening on {}", addr);

    loop {
        // Accept incoming connections
        let (socket, addr) = listener.accept().await?;
        info!("New RESP connection from {}", addr);

        // Clone the dispatcher Arc for this connection
        let dispatcher = dispatcher.clone();

        // Spawn a new task to handle this connection
        tokio::spawn(async move {
            let mut connection = Connection::new(socket);

            if let Err(e) = connection.handle(dispatcher).await {
                error!("Connection error from {}: {}", addr, e);
            }

            info!("Connection closed: {}", addr);
        });
    }
}

/// Run the server with a cluster manager
///
/// Allows distributing keys across multiple shards for better parallelism.
pub async fn run_with_cluster(
    addr: &str,
    cluster: Arc<ClusterManager>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Bind the TCP listener
    let listener = TcpListener::bind(addr).await?;
    info!("FerrumDB RESP server listening on {}", addr);

    loop {
        // Accept incoming connections
        let (socket, addr) = listener.accept().await?;
        info!("New RESP connection from {}", addr);

        // Clone the cluster Arc for this connection
        let cluster = cluster.clone();

        // Spawn a new task to handle this connection
        tokio::spawn(async move {
            let mut connection = Connection::new(socket);

            if let Err(e) = connection.handle_with_cluster(cluster).await {
                error!("Connection error from {}: {}", addr, e);
            }

            info!("Connection closed: {}", addr);
        });
    }
}
