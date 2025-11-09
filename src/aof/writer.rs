//! AOF writer
//!
//! Handles writing operations to the AOF file.

use super::{AofEntry, SyncPolicy};
use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::Path;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// AOF writer
pub struct AofWriter {
    file: Mutex<File>,
    sync_policy: SyncPolicy,
    last_sync: Mutex<Instant>,
}

impl AofWriter {
    /// Create a new AOF writer
    pub fn new<P: AsRef<Path>>(path: P, sync_policy: SyncPolicy) -> io::Result<Self> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;

        Ok(AofWriter {
            file: Mutex::new(file),
            sync_policy,
            last_sync: Mutex::new(Instant::now()),
        })
    }

    /// Write an entry to the AOF
    pub fn write(&self, entry: &AofEntry) -> io::Result<()> {
        let bytes = entry.to_bytes();

        let mut file = self.file.lock().unwrap();
        file.write_all(&bytes)?;

        // Apply sync policy
        match self.sync_policy {
            SyncPolicy::Always => {
                file.sync_all()?;
            }
            SyncPolicy::EverySecond => {
                let mut last_sync = self.last_sync.lock().unwrap();
                if last_sync.elapsed() >= Duration::from_secs(1) {
                    file.sync_all()?;
                    *last_sync = Instant::now();
                }
            }
            SyncPolicy::No => {
                // No explicit sync
            }
        }

        Ok(())
    }

    /// Force sync to disk
    pub fn sync(&self) -> io::Result<()> {
        let file = self.file.lock().unwrap();
        file.sync_all()?;
        *self.last_sync.lock().unwrap() = Instant::now();
        Ok(())
    }

    /// Flush buffered data
    pub fn flush(&self) -> io::Result<()> {
        let mut file = self.file.lock().unwrap();
        file.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aof::AofOperation;
    use bytes::Bytes;
    use std::fs;

    #[test]
    fn test_write_entry() {
        let temp_file = "test_aof_writer.aof";

        // Clean up if exists
        let _ = fs::remove_file(temp_file);

        let writer = AofWriter::new(temp_file, SyncPolicy::Always).unwrap();

        let entry = AofEntry::new(
            AofOperation::Set,
            Bytes::from("testkey"),
            vec![Bytes::from("testvalue")],
        );

        writer.write(&entry).unwrap();
        writer.sync().unwrap();

        // Verify file was created and has content
        let metadata = fs::metadata(temp_file).unwrap();
        assert!(metadata.len() > 0);

        // Clean up
        fs::remove_file(temp_file).unwrap();
    }
}
