//! Command execution module
//!
//! Provides a unified interface for all commands through the Command trait.
//! Each command is implemented in a separate file for high cohesion.

mod context;
mod registry;

// Command implementations
mod string;
mod key;
mod ttl;
mod counter;
mod list;
mod set;
mod hash;
mod admin;
mod search;

pub use context::CommandContext;
pub use registry::CommandRegistry;

use crate::protocol::RespValue;

/// Command execution trait
///
/// All commands implement this trait with a single execute method.
/// This provides loose coupling between command implementations and the dispatcher.
pub trait Command: Send + Sync {
    /// Execute the command with the given context and arguments
    ///
    /// Arguments:
    /// - ctx: mutable reference to the command context (contains the store)
    /// - args: command arguments (excluding the command name itself)
    ///
    /// Returns:
    /// - RespValue representing the response to send to the client
    fn execute(&self, ctx: &mut CommandContext, args: &[RespValue]) -> RespValue;

    /// Get the command name (for debugging/logging)
    fn name(&self) -> &'static str;

    /// Get the minimum number of arguments required
    fn min_args(&self) -> usize {
        0
    }

    /// Get the maximum number of arguments (None = unlimited)
    fn max_args(&self) -> Option<usize> {
        None
    }
}

/// Helper function to extract bulk string from RespValue
pub(crate) fn extract_bulk_string(value: &RespValue) -> Result<&bytes::Bytes, &'static str> {
    value.as_bulk_string().ok_or("Expected bulk string")
}

/// Helper function to extract integer from RespValue or parse from bulk string
pub(crate) fn extract_integer(value: &RespValue) -> Result<i64, &'static str> {
    match value {
        RespValue::Integer(i) => Ok(*i),
        RespValue::BulkString(bytes) => {
            let s = std::str::from_utf8(bytes).map_err(|_| "Invalid UTF-8")?;
            s.parse::<i64>().map_err(|_| "Invalid integer")
        }
        _ => Err("Expected integer or bulk string"),
    }
}

/// Helper function to log an operation to AOF
pub(crate) fn log_to_aof(
    ctx: &CommandContext,
    op: crate::aof::AofOperation,
    key: bytes::Bytes,
    payload: Vec<bytes::Bytes>,
) {
    use tracing::warn;

    if let Some(ref aof_writer) = ctx.aof_writer {
        let entry = crate::aof::AofEntry::new(op, key, payload);
        if let Err(e) = aof_writer.write(&entry) {
            warn!("Failed to write to AOF: {}", e);
        }
    }
}
