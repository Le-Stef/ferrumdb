//! Shard implementation
//!
//! Each shard runs in its own thread with a dedicated MemoryStore.
//! This provides true parallelism while maintaining single-threaded consistency
//! within each shard.

use crate::aof::{AofWriter, AofReader, SyncPolicy};
use crate::commands::{CommandContext, CommandRegistry};
use crate::protocol::RespValue;
use crate::store::{MemoryStore, StoreStats};
use tokio::sync::{mpsc, oneshot};
use std::sync::Arc;
use std::path::PathBuf;
use tracing::{debug, error, info, warn};

/// Configuration for a shard
#[derive(Debug, Clone)]
pub struct ShardConfig {
    /// Unique shard identifier
    pub shard_id: usize,

    /// Enable AOF persistence
    pub aof_enabled: bool,

    /// Path to AOF file (if enabled)
    pub aof_path: Option<String>,
}

/// A command sent to a shard
pub struct ShardCommand {
    /// The RESP command to execute
    pub command: RespValue,

    /// Channel to send the response back
    pub response_tx: oneshot::Sender<RespValue>,
}

/// A shard that processes commands in its own thread
pub struct Shard {
    /// Shard ID
    id: usize,

    /// Channel to send commands to the shard thread
    command_tx: mpsc::UnboundedSender<ShardCommand>,

    /// Channel to request stats
    stats_tx: mpsc::UnboundedSender<oneshot::Sender<StoreStats>>,
}

impl Shard {
    /// Create a new shard and start its thread
    pub fn new(config: ShardConfig) -> anyhow::Result<Self> {
        let shard_id = config.shard_id;
        info!("Initializing shard {}", shard_id);

        // Create channels
        let (command_tx, command_rx) = mpsc::unbounded_channel();
        let (stats_tx, stats_rx) = mpsc::unbounded_channel();

        // Spawn the shard thread
        std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to create shard runtime");

            runtime.block_on(async move {
                if let Err(e) = Self::run_shard_loop(config, command_rx, stats_rx).await {
                    error!("Shard {} failed: {}", shard_id, e);
                }
            });
        });

        info!("Shard {} started", shard_id);

        Ok(Shard {
            id: shard_id,
            command_tx,
            stats_tx,
        })
    }

    /// Send a command to this shard
    pub async fn send_command(&self, command: ShardCommand) -> anyhow::Result<()> {
        self.command_tx
            .send(command)
            .map_err(|_| anyhow::anyhow!("Shard {} channel closed", self.id))
    }

    /// Get statistics from this shard
    pub async fn get_stats(&self) -> StoreStats {
        let (tx, rx) = oneshot::channel();

        if self.stats_tx.send(tx).is_err() {
            error!("Failed to request stats from shard {}", self.id);
            return StoreStats {
                total_keys: 0,
                expired_keys: 0,
                active_keys: 0,
                used_memory_bytes: 0,
            };
        }

        rx.await.unwrap_or_else(|_| StoreStats {
            total_keys: 0,
            expired_keys: 0,
            active_keys: 0,
            used_memory_bytes: 0,
        })
    }

    /// The main loop that runs in the shard's thread
    async fn run_shard_loop(
        config: ShardConfig,
        mut command_rx: mpsc::UnboundedReceiver<ShardCommand>,
        mut stats_rx: mpsc::UnboundedReceiver<oneshot::Sender<StoreStats>>,
    ) -> anyhow::Result<()> {
        let shard_id = config.shard_id;
        info!("Shard {} loop starting", shard_id);

        // Initialize the store
        let store = MemoryStore::new();

        // Initialize AOF writer if enabled
        let aof_writer = if config.aof_enabled {
            if let Some(aof_path) = config.aof_path {
                info!("Shard {}: Initializing AOF at {}", shard_id, aof_path);

                let path = PathBuf::from(&aof_path);

                // Load existing AOF if present
                let entries = match AofReader::load(&path) {
                    Ok(reader) => {
                        let entries = reader.parse_entries();
                        info!("Shard {}: Loaded {} AOF entries", shard_id, entries.len());
                        entries
                    }
                    Err(e) => {
                        info!("Shard {}: No existing AOF or error loading: {}", shard_id, e);
                        Vec::new()
                    }
                };

                // Create writer
                let writer = match AofWriter::new(&path, SyncPolicy::EverySecond) {
                    Ok(w) => w,
                    Err(e) => {
                        error!("Shard {}: Failed to create AOF writer: {}", shard_id, e);
                        return Err(anyhow::anyhow!("Failed to create AOF writer: {}", e));
                    }
                };

                // Replay entries if any
                if !entries.is_empty() {
                    // We'll need to replay these entries
                    // For now, skip replay in shard (will implement later)
                    warn!("Shard {}: AOF replay not yet implemented in sharded mode ({} entries skipped)", shard_id, entries.len());
                }

                Some(Arc::new(writer))
            } else {
                None
            }
        } else {
            None
        };

        // Create command context
        let mut context = CommandContext {
            store,
            aof_writer,
        };

        // Create command registry
        let registry = CommandRegistry::new();

        // Main event loop
        loop {
            tokio::select! {
                // Process commands
                Some(shard_command) = command_rx.recv() => {
                    debug!("Shard {} received command: {:?}", shard_id, shard_command.command);

                    // Dispatch the command
                    let response = Self::dispatch_command(&registry, &mut context, shard_command.command);

                    // Send response back
                    let _ = shard_command.response_tx.send(response);
                }

                // Handle stats requests
                Some(stats_tx) = stats_rx.recv() => {
                    let stats = context.store.stats();
                    let _ = stats_tx.send(stats);
                }

                // Channel closed, exit
                else => {
                    info!("Shard {} shutting down", shard_id);
                    break;
                }
            }
        }

        Ok(())
    }

    /// Dispatch a command using the registry
    fn dispatch_command(
        registry: &CommandRegistry,
        context: &mut CommandContext,
        command: RespValue,
    ) -> RespValue {
        use base64::{Engine as _, engine::general_purpose};

        // Parse command array
        let parts = match command {
            RespValue::Array(ref parts) if !parts.is_empty() => parts,
            ref invalid => {
                // Encode the invalid command in Base64 for debugging
                let debug_msg = format!("{:?}", invalid);
                let b64 = general_purpose::STANDARD.encode(debug_msg.as_bytes());
                error!("Invalid command format - not an array or empty. Command (B64): {}", b64);
                return RespValue::error("ERR invalid command format");
            }
        };

        // Extract command name
        let cmd_name = match &parts[0] {
            RespValue::BulkString(name) => match std::str::from_utf8(name) {
                Ok(s) => s,
                Err(_) => {
                    let b64 = general_purpose::STANDARD.encode(name);
                    error!("Invalid command name encoding. Raw bytes (B64): {}", b64);
                    return RespValue::error("ERR invalid command name encoding");
                }
            },
            invalid => {
                let debug_msg = format!("{:?}", invalid);
                let b64 = general_purpose::STANDARD.encode(debug_msg.as_bytes());
                error!("Command name must be bulk string. Received (B64): {}", b64);
                return RespValue::error("ERR command name must be a bulk string");
            }
        };

        // Get command from registry
        let cmd = match registry.get(cmd_name) {
            Some(c) => c,
            None => {
                warn!("Unknown command: '{}'", cmd_name);
                return RespValue::error(format!("ERR unknown command '{}'", cmd_name));
            }
        };

        // Get arguments (everything after command name)
        let args = &parts[1..];

        // Execute command
        cmd.execute(context, args)
    }

    /// Get shard ID
    pub fn id(&self) -> usize {
        self.id
    }
}

impl Drop for Shard {
    fn drop(&mut self) {
        info!("Shard {} dropped", self.id);
    }
}
