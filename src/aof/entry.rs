//! AOF entry format
//!
//! Binary format: [op_type(u8)] [timestamp(u64)] [key_len(u32)] [key_bytes] [payload...] [checksum(u64)]

use bytes::Bytes;
use std::time::{SystemTime, UNIX_EPOCH};

/// AOF operation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum AofOperation {
    /// SET operation
    Set = 1,
    /// DELETE operation
    Del = 2,
    /// EXPIRE operation
    Expire = 3,
    /// HSET operation (hash field set)
    HSet = 4,
    /// HDEL operation (hash field delete)
    HDel = 5,
    /// LPUSH operation
    LPush = 6,
    /// RPUSH operation
    RPush = 7,
    /// SADD operation
    SAdd = 8,
    /// INCR operation
    Incr = 9,
    /// INCRBY operation
    IncrBy = 10,
}

impl AofOperation {
    /// Convert from u8
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            1 => Some(AofOperation::Set),
            2 => Some(AofOperation::Del),
            3 => Some(AofOperation::Expire),
            4 => Some(AofOperation::HSet),
            5 => Some(AofOperation::HDel),
            6 => Some(AofOperation::LPush),
            7 => Some(AofOperation::RPush),
            8 => Some(AofOperation::SAdd),
            9 => Some(AofOperation::Incr),
            10 => Some(AofOperation::IncrBy),
            _ => None,
        }
    }
}

/// AOF entry
#[derive(Debug, Clone)]
pub struct AofEntry {
    /// Operation type
    pub op: AofOperation,
    /// Timestamp (milliseconds since UNIX epoch)
    pub timestamp: u64,
    /// Key
    pub key: Bytes,
    /// Operation payload (depends on operation type)
    pub payload: Vec<Bytes>,
}

impl AofEntry {
    /// Create a new AOF entry
    pub fn new(op: AofOperation, key: Bytes, payload: Vec<Bytes>) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        AofEntry {
            op,
            timestamp,
            key,
            payload,
        }
    }

    /// Serialize to bytes with checksum
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::new();

        // Operation type (1 byte)
        buf.push(self.op as u8);

        // Timestamp (8 bytes)
        buf.extend_from_slice(&self.timestamp.to_le_bytes());

        // Key length (4 bytes) + key bytes
        buf.extend_from_slice(&(self.key.len() as u32).to_le_bytes());
        buf.extend_from_slice(&self.key);

        // Payload count (4 bytes)
        buf.extend_from_slice(&(self.payload.len() as u32).to_le_bytes());

        // Each payload: length (4 bytes) + bytes
        for item in &self.payload {
            buf.extend_from_slice(&(item.len() as u32).to_le_bytes());
            buf.extend_from_slice(item);
        }

        // Checksum (8 bytes) - xxhash64 of all previous bytes
        let checksum = xxhash_rust::xxh64::xxh64(&buf, 0);
        buf.extend_from_slice(&checksum.to_le_bytes());

        buf
    }

    /// Deserialize from bytes with checksum verification
    pub fn from_bytes(data: &[u8]) -> Result<(Self, usize), String> {
        if data.len() < 17 {
            // Minimum: 1 (op) + 8 (ts) + 4 (key_len) + 0 (key) + 4 (payload_count) + 8 (checksum)
            return Err("Insufficient data".to_string());
        }

        let mut pos = 0;

        // Operation type
        let op = AofOperation::from_u8(data[pos])
            .ok_or("Invalid operation type")?;
        pos += 1;

        // Timestamp
        let timestamp = u64::from_le_bytes(
            data[pos..pos + 8].try_into().map_err(|_| "Invalid timestamp")?
        );
        pos += 8;

        // Key length and key
        let key_len = u32::from_le_bytes(
            data[pos..pos + 4].try_into().map_err(|_| "Invalid key length")?
        ) as usize;
        pos += 4;

        if pos + key_len > data.len() {
            return Err("Invalid key length".to_string());
        }

        let key = Bytes::copy_from_slice(&data[pos..pos + key_len]);
        pos += key_len;

        // Payload count
        if pos + 4 > data.len() {
            return Err("Missing payload count".to_string());
        }

        let payload_count = u32::from_le_bytes(
            data[pos..pos + 4].try_into().map_err(|_| "Invalid payload count")?
        ) as usize;
        pos += 4;

        // Read payloads
        let mut payload = Vec::with_capacity(payload_count);
        for _ in 0..payload_count {
            if pos + 4 > data.len() {
                return Err("Missing payload length".to_string());
            }

            let item_len = u32::from_le_bytes(
                data[pos..pos + 4].try_into().map_err(|_| "Invalid payload item length")?
            ) as usize;
            pos += 4;

            if pos + item_len > data.len() {
                return Err("Invalid payload item length".to_string());
            }

            payload.push(Bytes::copy_from_slice(&data[pos..pos + item_len]));
            pos += item_len;
        }

        // Checksum verification
        if pos + 8 > data.len() {
            return Err("Missing checksum".to_string());
        }

        let stored_checksum = u64::from_le_bytes(
            data[pos..pos + 8].try_into().map_err(|_| "Invalid checksum")?
        );
        pos += 8;

        // Verify checksum (everything except the checksum itself)
        let calculated_checksum = xxhash_rust::xxh64::xxh64(&data[..pos - 8], 0);
        if stored_checksum != calculated_checksum {
            return Err(format!(
                "Checksum mismatch: expected {}, got {}",
                stored_checksum, calculated_checksum
            ));
        }

        Ok((
            AofEntry {
                op,
                timestamp,
                key,
                payload,
            },
            pos,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_deserialize_set() {
        let entry = AofEntry::new(
            AofOperation::Set,
            Bytes::from("mykey"),
            vec![Bytes::from("myvalue")],
        );

        let bytes = entry.to_bytes();
        let (decoded, size) = AofEntry::from_bytes(&bytes).unwrap();

        assert_eq!(size, bytes.len());
        assert_eq!(decoded.op, AofOperation::Set);
        assert_eq!(decoded.key, Bytes::from("mykey"));
        assert_eq!(decoded.payload.len(), 1);
        assert_eq!(decoded.payload[0], Bytes::from("myvalue"));
    }

    #[test]
    fn test_serialize_deserialize_hset() {
        let entry = AofEntry::new(
            AofOperation::HSet,
            Bytes::from("myhash"),
            vec![Bytes::from("field1"), Bytes::from("value1")],
        );

        let bytes = entry.to_bytes();
        let (decoded, _) = AofEntry::from_bytes(&bytes).unwrap();

        assert_eq!(decoded.op, AofOperation::HSet);
        assert_eq!(decoded.key, Bytes::from("myhash"));
        assert_eq!(decoded.payload.len(), 2);
        assert_eq!(decoded.payload[0], Bytes::from("field1"));
        assert_eq!(decoded.payload[1], Bytes::from("value1"));
    }

    #[test]
    fn test_checksum_validation() {
        let entry = AofEntry::new(
            AofOperation::Set,
            Bytes::from("key"),
            vec![Bytes::from("value")],
        );

        let mut bytes = entry.to_bytes();

        // Corrupt the checksum
        let len = bytes.len();
        bytes[len - 1] ^= 0xFF;

        let result = AofEntry::from_bytes(&bytes);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Checksum mismatch"));
    }
}
