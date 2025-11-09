//! Append-Only File (AOF) persistence module
//!
//! Provides durability by logging all write operations to disk.
//! Each operation is written in a compact binary format with checksums.

mod entry;
mod writer;
mod reader;
mod replay;

pub use entry::{AofEntry, AofOperation};
pub use writer::AofWriter;
pub use reader::AofReader;
pub use replay::replay_entries;

use std::path::PathBuf;

/// AOF sync policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncPolicy {
    /// Sync after every write (safest, slowest)
    Always,
    /// Sync every second (balanced)
    EverySecond,
    /// Let the OS decide when to sync (fastest, least safe)
    No,
}

impl Default for SyncPolicy {
    fn default() -> Self {
        SyncPolicy::EverySecond
    }
}

/// AOF configuration
#[derive(Debug, Clone)]
pub struct AofConfig {
    /// Path to the AOF file
    pub path: PathBuf,
    /// Sync policy
    pub sync_policy: SyncPolicy,
    /// Whether to enable AOF
    pub enabled: bool,
}

impl Default for AofConfig {
    fn default() -> Self {
        AofConfig {
            path: PathBuf::from("ferrumdb.aof"),
            sync_policy: SyncPolicy::default(),
            enabled: true,
        }
    }
}
