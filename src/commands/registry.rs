//! Command registry
//!
//! Centralized registry for all available commands.
//! This allows loose coupling between command implementations and the dispatcher.

use super::{Command, string, key, ttl, counter, list, set, hash, admin, search};
use std::collections::HashMap;
use std::sync::Arc;

/// Registry of all available commands
pub struct CommandRegistry {
    commands: HashMap<String, Arc<dyn Command>>,
}

impl CommandRegistry {
    /// Create a new command registry and register all commands
    pub fn new() -> Self {
        let mut registry = CommandRegistry {
            commands: HashMap::new(),
        };

        // Register string commands
        registry.register(Arc::new(string::SetCommand));
        registry.register(Arc::new(string::GetCommand));

        // Register key commands
        registry.register(Arc::new(key::DelCommand));
        registry.register(Arc::new(key::ExistsCommand));

        // Register TTL commands
        registry.register(Arc::new(ttl::ExpireCommand));
        registry.register(Arc::new(ttl::TtlCommand));

        // Register counter commands
        registry.register(Arc::new(counter::IncrCommand));
        registry.register(Arc::new(counter::IncrByCommand));
        registry.register(Arc::new(counter::DecrCommand));
        registry.register(Arc::new(counter::DecrByCommand));

        // Register list commands
        registry.register(Arc::new(list::LPushCommand));
        registry.register(Arc::new(list::RPushCommand));
        registry.register(Arc::new(list::LRangeCommand));
        registry.register(Arc::new(list::LLenCommand));

        // Register set commands
        registry.register(Arc::new(set::SAddCommand));
        registry.register(Arc::new(set::SMembersCommand));
        registry.register(Arc::new(set::SCardCommand));

        // Register hash commands
        registry.register(Arc::new(hash::HSetCommand));
        registry.register(Arc::new(hash::HGetCommand));
        registry.register(Arc::new(hash::HGetAllCommand));
        registry.register(Arc::new(hash::HDelCommand));
        registry.register(Arc::new(hash::HKeysCommand));
        registry.register(Arc::new(hash::HIncrByCommand));

        // Register admin commands
        registry.register(Arc::new(admin::InfoCommand));
        registry.register(Arc::new(admin::FlushDbCommand));
        registry.register(Arc::new(admin::ClientCommand));

        // Register search commands
        registry.register(Arc::new(search::KeysCommand));

        registry
    }

    /// Register a command
    fn register(&mut self, command: Arc<dyn Command>) {
        let name = command.name().to_uppercase();
        self.commands.insert(name, command);
    }

    /// Get a command by name (case-insensitive)
    pub fn get(&self, name: &str) -> Option<Arc<dyn Command>> {
        self.commands.get(&name.to_uppercase()).cloned()
    }

    /// Check if a command exists
    pub fn has_command(&self, name: &str) -> bool {
        self.commands.contains_key(&name.to_uppercase())
    }

    /// Get all command names
    pub fn command_names(&self) -> Vec<String> {
        self.commands.keys().cloned().collect()
    }
}

impl Default for CommandRegistry {
    fn default() -> Self {
        Self::new()
    }
}
