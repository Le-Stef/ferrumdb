//! In-memory storage module
//!
//! Provides the core data structures for storing key-value pairs in memory.
//! This module is independent of protocol and command handling (loose coupling).

mod entry;
mod value;
mod memory;

pub use entry::Entry;
pub use value::Value;
pub use memory::{MemoryStore, StoreStats};
