//! AOF reader
//!
//! Handles reading and replaying operations from the AOF file.

use super::AofEntry;
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;
use tracing::{info, warn, error};

/// AOF reader
pub struct AofReader {
    data: Vec<u8>,
}

impl AofReader {
    /// Load AOF file
    pub fn load<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let mut file = File::open(path)?;
        let mut data = Vec::new();
        file.read_to_end(&mut data)?;

        Ok(AofReader { data })
    }

    /// Parse all entries from the AOF
    ///
    /// Returns a vector of successfully parsed entries.
    /// Corrupted entries are logged and skipped.
    pub fn parse_entries(&self) -> Vec<AofEntry> {
        let mut entries = Vec::new();
        let mut pos = 0;
        let mut entry_count = 0;
        let mut error_count = 0;

        while pos < self.data.len() {
            match AofEntry::from_bytes(&self.data[pos..]) {
                Ok((entry, size)) => {
                    entries.push(entry);
                    pos += size;
                    entry_count += 1;
                }
                Err(e) => {
                    error!("Failed to parse AOF entry at position {}: {}", pos, e);
                    error_count += 1;
                    // Try to skip ahead to find the next valid entry
                    // For now, we stop at the first error to avoid corruption
                    break;
                }
            }
        }

        if error_count > 0 {
            warn!("AOF parsing completed with {} errors. {} entries recovered.", error_count, entry_count);
        } else {
            info!("AOF loaded successfully: {} entries", entry_count);
        }

        entries
    }

    /// Get the total size of the AOF data
    pub fn size(&self) -> usize {
        self.data.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aof::{AofOperation, AofWriter, SyncPolicy};
    use bytes::Bytes;
    use std::fs;

    #[test]
    fn test_load_and_parse() {
        let temp_file = "test_aof_reader.aof";

        // Clean up if exists
        let _ = fs::remove_file(temp_file);

        // Write some entries
        let writer = AofWriter::new(temp_file, SyncPolicy::Always).unwrap();

        let entry1 = AofEntry::new(
            AofOperation::Set,
            Bytes::from("key1"),
            vec![Bytes::from("value1")],
        );
        writer.write(&entry1).unwrap();

        let entry2 = AofEntry::new(
            AofOperation::Set,
            Bytes::from("key2"),
            vec![Bytes::from("value2")],
        );
        writer.write(&entry2).unwrap();

        writer.sync().unwrap();

        // Read back
        let reader = AofReader::load(temp_file).unwrap();
        let entries = reader.parse_entries();

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].key, Bytes::from("key1"));
        assert_eq!(entries[1].key, Bytes::from("key2"));

        // Clean up
        fs::remove_file(temp_file).unwrap();
    }
}
