//! Command execution context

use crate::store::MemoryStore;
use crate::aof::AofWriter;
use std::sync::Arc;

/// Context provided to commands during execution
///
/// This context gives commands access to the store and any other
/// resources they need. Keeps coupling loose by providing a clean interface.
pub struct CommandContext {
    /// The memory store
    pub store: MemoryStore,

    /// Optional AOF writer for persistence
    pub aof_writer: Option<Arc<AofWriter>>,
}

impl CommandContext {
    /// Create a new command context
    pub fn new() -> Self {
        CommandContext {
            store: MemoryStore::new(),
            aof_writer: None,
        }
    }

    /// Create a context with a specific store capacity
    pub fn with_capacity(capacity: usize) -> Self {
        CommandContext {
            store: MemoryStore::with_capacity(capacity),
            aof_writer: None,
        }
    }

    /// Set the AOF writer
    pub fn set_aof_writer(&mut self, writer: Arc<AofWriter>) {
        self.aof_writer = Some(writer);
    }
}

impl Default for CommandContext {
    fn default() -> Self {
        Self::new()
    }
}
