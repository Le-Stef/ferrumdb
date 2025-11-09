//! FerrumDB - A lightweight, high-performance in-memory key-value store
//!
//! FerrumDB is designed with strong cohesion and loose coupling principles:
//! - Each module has a single, well-defined responsibility
//! - Modules communicate through clear, minimal interfaces
//! - No circular dependencies between modules

pub mod protocol;
pub mod store;
pub mod commands;
pub mod dispatch;
pub mod server;
pub mod aof;
pub mod web;
pub mod cluster;

/// Re-export commonly used types
pub use store::{MemoryStore, Entry};
pub use protocol::{RespValue, RespError};
pub use commands::{Command, CommandContext};
pub use cluster::{ClusterManager, Shard};
