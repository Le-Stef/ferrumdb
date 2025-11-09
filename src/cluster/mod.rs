//! Cluster and sharding module
//!
//! This module implements the sharding logic for distributing keys across multiple
//! shards, each running in its own thread. This architecture is designed to support
//! future multi-node clustering.

mod shard;
mod router;

pub use shard::{Shard, ShardCommand, ShardConfig};
pub use router::ShardRouter;

use crate::protocol::RespValue;
use std::sync::Arc;
use tokio::sync::oneshot;
use tracing::{info, error};

/// Cluster manager that owns all shards and routes commands
pub struct ClusterManager {
    shards: Vec<Arc<Shard>>,
    router: ShardRouter,
}

impl ClusterManager {
    /// Create a new cluster manager with the specified number of shards
    pub fn new(num_shards: usize, aof_enabled: bool) -> anyhow::Result<Self> {
        info!("Initializing cluster with {} shards", num_shards);

        let mut shards = Vec::with_capacity(num_shards);

        for shard_id in 0..num_shards {
            let config = ShardConfig {
                shard_id,
                aof_enabled,
                aof_path: if aof_enabled {
                    Some(format!("ferrumdb_shard_{}.aof", shard_id))
                } else {
                    None
                },
            };

            let shard = Shard::new(config)?;
            shards.push(Arc::new(shard));
        }

        let router = ShardRouter::new(num_shards);

        info!("Cluster initialized with {} shards", num_shards);

        Ok(ClusterManager { shards, router })
    }

    /// Execute a command on the appropriate shard
    pub async fn execute(&self, command: RespValue) -> RespValue {
        // Extract the key from the command to determine the shard
        let shard_id = match self.extract_key_and_route(&command) {
            Some(id) => id,
            None => {
                // Commands without keys (like INFO, FLUSHDB) go to shard 0
                0
            }
        };

        // Get the shard
        let shard = &self.shards[shard_id];

        // Create a oneshot channel for the response
        let (tx, rx) = oneshot::channel();

        // Send command to shard
        let shard_command = ShardCommand {
            command,
            response_tx: tx,
        };

        if let Err(e) = shard.send_command(shard_command).await {
            error!("Failed to send command to shard {}: {}", shard_id, e);
            return RespValue::error("ERR internal error");
        }

        // Wait for response
        match rx.await {
            Ok(response) => response,
            Err(_) => {
                error!("Shard {} did not respond", shard_id);
                RespValue::error("ERR shard did not respond")
            }
        }
    }

    /// Extract the key from a command and route to shard
    fn extract_key_and_route(&self, command: &RespValue) -> Option<usize> {
        if let RespValue::Array(parts) = command {
            if parts.len() < 2 {
                return None;
            }

            // Get the command name
            let cmd_name = match &parts[0] {
                RespValue::BulkString(b) => std::str::from_utf8(b).ok()?,
                _ => return None,
            };

            // Commands without keys
            let no_key_commands = ["INFO", "FLUSHDB", "PING"];
            if no_key_commands.contains(&cmd_name.to_uppercase().as_str()) {
                return None;
            }

            // Extract the key (second element for most commands)
            let key = match &parts[1] {
                RespValue::BulkString(b) => b,
                _ => return None,
            };

            Some(self.router.route_key(key))
        } else {
            None
        }
    }

    /// Get statistics from all shards
    pub async fn get_cluster_stats(&self) -> ClusterStats {
        let mut total_keys = 0;
        let mut total_memory = 0;

        for shard in &self.shards {
            let stats = shard.get_stats().await;
            total_keys += stats.active_keys;
            total_memory += stats.used_memory_bytes;
        }

        ClusterStats {
            num_shards: self.shards.len(),
            total_keys,
            total_memory_bytes: total_memory,
        }
    }

    /// Get detailed statistics for each shard
    pub async fn get_shard_details(&self) -> Vec<ShardStats> {
        let mut shard_stats = Vec::new();

        for shard in &self.shards {
            let store_stats = shard.get_stats().await;
            shard_stats.push(ShardStats {
                shard_id: shard.id(),
                active_keys: store_stats.active_keys,
                total_keys: store_stats.total_keys,
                expired_keys: store_stats.expired_keys,
                memory_bytes: store_stats.used_memory_bytes,
            });
        }

        shard_stats
    }

    /// Get number of shards
    pub fn num_shards(&self) -> usize {
        self.shards.len()
    }
}

/// Cluster statistics
#[derive(Debug, Clone)]
pub struct ClusterStats {
    pub num_shards: usize,
    pub total_keys: usize,
    pub total_memory_bytes: usize,
}

/// Statistics for a single shard
#[derive(Debug, Clone, serde::Serialize)]
pub struct ShardStats {
    pub shard_id: usize,
    pub active_keys: usize,
    pub total_keys: usize,
    pub expired_keys: usize,
    pub memory_bytes: usize,
}
