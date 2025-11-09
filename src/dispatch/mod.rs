//! Command dispatcher
//!
//! Routes incoming commands to the appropriate handler.
//! This module provides loose coupling between the server and command implementations.

use crate::commands::{CommandContext, CommandRegistry};
use crate::protocol::RespValue;
use crate::aof::{AofConfig, AofWriter, AofReader, replay_entries};
use std::sync::Arc;
use std::path::Path;
use tracing::{debug, warn, info};

/// Command dispatcher
///
/// Receives RESP commands, validates them, and routes to appropriate handlers
pub struct Dispatcher {
    /// Command registry
    registry: CommandRegistry,

    /// Command execution context
    context: CommandContext,
}

impl Dispatcher {
    /// Create a new dispatcher
    pub fn new() -> Self {
        Dispatcher {
            registry: CommandRegistry::new(),
            context: CommandContext::new(),
        }
    }

    /// Create a dispatcher with specified store capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Dispatcher {
            registry: CommandRegistry::new(),
            context: CommandContext::with_capacity(capacity),
        }
    }

    /// Create a dispatcher with AOF support
    pub fn with_aof(config: AofConfig) -> std::io::Result<Self> {
        let mut context = CommandContext::new();

        // Load and replay AOF if it exists
        if config.enabled && Path::new(&config.path).exists() {
            info!("Loading AOF from {:?}", config.path);
            match AofReader::load(&config.path) {
                Ok(reader) => {
                    let entries = reader.parse_entries();
                    info!("Found {} AOF entries", entries.len());
                    match replay_entries(&mut context.store, entries) {
                        Ok(count) => info!("Replayed {} entries from AOF", count),
                        Err(e) => warn!("Error replaying AOF: {}", e),
                    }
                }
                Err(e) => warn!("Failed to load AOF: {}", e),
            }
        }

        // Initialize AOF writer
        if config.enabled {
            let writer = AofWriter::new(&config.path, config.sync_policy)?;
            context.set_aof_writer(Arc::new(writer));
            info!("AOF writer initialized at {:?}", config.path);
        }

        Ok(Dispatcher {
            registry: CommandRegistry::new(),
            context,
        })
    }

    /// Dispatch a command
    ///
    /// Takes a RESP value (expected to be an array), extracts the command name
    /// and arguments, then routes to the appropriate handler.
    pub fn dispatch(&mut self, value: RespValue) -> RespValue {
        // Commands should be arrays
        let args = match value.as_array() {
            Some(arr) if !arr.is_empty() => arr,
            Some(_) => {
                return RespValue::error("ERR empty command array");
            }
            None => {
                return RespValue::error("ERR expected array");
            }
        };

        // First element is the command name
        let cmd_name = match args[0].as_bulk_string() {
            Some(name) => match std::str::from_utf8(name) {
                Ok(s) => s,
                Err(_) => {
                    return RespValue::error("ERR invalid command name encoding");
                }
            },
            None => {
                return RespValue::error("ERR command name must be a bulk string");
            }
        };

        debug!("Dispatching command: {}", cmd_name);

        // Look up the command
        let command = match self.registry.get(cmd_name) {
            Some(cmd) => cmd,
            None => {
                warn!("Unknown command: {}", cmd_name);
                return RespValue::error(format!("ERR unknown command '{}'", cmd_name));
            }
        };

        // Extract arguments (everything after the command name)
        let cmd_args = &args[1..];

        // Validate argument count
        if cmd_args.len() < command.min_args() {
            return RespValue::error(format!(
                "ERR wrong number of arguments for '{}' command",
                cmd_name
            ));
        }

        if let Some(max) = command.max_args() {
            if cmd_args.len() > max {
                return RespValue::error(format!(
                    "ERR wrong number of arguments for '{}' command",
                    cmd_name
                ));
            }
        }

        // Execute the command
        command.execute(&mut self.context, cmd_args)
    }

    /// Get reference to the context (for testing/inspection)
    pub fn context(&self) -> &CommandContext {
        &self.context
    }

    /// Get mutable reference to the context (for testing/inspection)
    pub fn context_mut(&mut self) -> &mut CommandContext {
        &mut self.context
    }
}

impl Default for Dispatcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;

    #[test]
    fn test_dispatch_set_get() {
        let mut dispatcher = Dispatcher::new();

        // SET mykey myvalue
        let set_cmd = RespValue::array(vec![
            RespValue::bulk_string("SET"),
            RespValue::bulk_string("mykey"),
            RespValue::bulk_string("myvalue"),
        ]);

        let result = dispatcher.dispatch(set_cmd);
        assert_eq!(result, RespValue::simple_string("OK"));

        // GET mykey
        let get_cmd = RespValue::array(vec![
            RespValue::bulk_string("GET"),
            RespValue::bulk_string("mykey"),
        ]);

        let result = dispatcher.dispatch(get_cmd);
        assert_eq!(result, RespValue::bulk_string(Bytes::from("myvalue")));
    }

    #[test]
    fn test_dispatch_unknown_command() {
        let mut dispatcher = Dispatcher::new();

        let cmd = RespValue::array(vec![
            RespValue::bulk_string("UNKNOWN"),
        ]);

        let result = dispatcher.dispatch(cmd);
        assert!(matches!(result, RespValue::Error(_)));
    }

    #[test]
    fn test_dispatch_invalid_args() {
        let mut dispatcher = Dispatcher::new();

        // GET without key
        let cmd = RespValue::array(vec![
            RespValue::bulk_string("GET"),
        ]);

        let result = dispatcher.dispatch(cmd);
        assert!(matches!(result, RespValue::Error(_)));
    }
}
