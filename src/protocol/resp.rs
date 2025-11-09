//! RESP2 parser and encoder
//!
//! Implements parsing from bytes to RespValue and encoding from RespValue to bytes

use super::types::{RespValue, RespError};
use bytes::{Buf, BufMut, Bytes, BytesMut};

const CRLF: &[u8] = b"\r\n";

/// RESP2 Parser
pub struct RespParser;

impl RespParser {
    /// Parse a RESP value from a buffer
    ///
    /// Returns Ok(Some(value)) if a complete value was parsed,
    /// Ok(None) if more data is needed,
    /// Err(e) if parsing failed
    pub fn parse(buf: &mut BytesMut) -> Result<Option<RespValue>, RespError> {
        if buf.is_empty() {
            return Ok(None);
        }

        let first_byte = buf[0];

        match first_byte {
            b'+' => Self::parse_simple_string(buf),
            b'-' => Self::parse_error(buf),
            b':' => Self::parse_integer(buf),
            b'$' => Self::parse_bulk_string(buf),
            b'*' => Self::parse_array(buf),
            _ => Err(RespError::InvalidProtocol(
                format!("Unknown type prefix: {}", first_byte as char)
            )),
        }
    }

    /// Parse simple string: +OK\r\n
    fn parse_simple_string(buf: &mut BytesMut) -> Result<Option<RespValue>, RespError> {
        if let Some(line) = Self::read_line(buf)? {
            let s = String::from_utf8(line[1..].to_vec())
                .map_err(|_| RespError::InvalidUtf8)?;
            Ok(Some(RespValue::SimpleString(s)))
        } else {
            Ok(None)
        }
    }

    /// Parse error: -Error message\r\n
    fn parse_error(buf: &mut BytesMut) -> Result<Option<RespValue>, RespError> {
        if let Some(line) = Self::read_line(buf)? {
            let s = String::from_utf8(line[1..].to_vec())
                .map_err(|_| RespError::InvalidUtf8)?;
            Ok(Some(RespValue::Error(s)))
        } else {
            Ok(None)
        }
    }

    /// Parse integer: :1000\r\n
    fn parse_integer(buf: &mut BytesMut) -> Result<Option<RespValue>, RespError> {
        if let Some(line) = Self::read_line(buf)? {
            let s = std::str::from_utf8(&line[1..])
                .map_err(|_| RespError::InvalidUtf8)?;
            let i = s.parse::<i64>()
                .map_err(|_| RespError::IntegerOverflow)?;
            Ok(Some(RespValue::Integer(i)))
        } else {
            Ok(None)
        }
    }

    /// Parse bulk string: $6\r\nfoobar\r\n or $-1\r\n (null)
    fn parse_bulk_string(buf: &mut BytesMut) -> Result<Option<RespValue>, RespError> {
        // First line contains the length
        if let Some(line) = Self::peek_line(buf)? {
            let s = std::str::from_utf8(&line[1..])
                .map_err(|_| RespError::InvalidUtf8)?;
            let len = s.parse::<i64>()
                .map_err(|_| RespError::IntegerOverflow)?;

            if len == -1 {
                // Null bulk string
                Self::read_line(buf)?; // consume the line
                return Ok(Some(RespValue::Null));
            }

            if len < 0 {
                return Err(RespError::InvalidProtocol(
                    format!("Invalid bulk string length: {}", len)
                ));
            }

            let total_len = line.len() + 2 + len as usize + 2; // $len\r\n + data + \r\n

            if buf.len() < total_len {
                // Not enough data yet
                return Ok(None);
            }

            // Consume the length line
            Self::read_line(buf)?;

            // Read the actual data
            let data = buf.split_to(len as usize);

            // Verify and consume the trailing CRLF
            if buf.len() < 2 || &buf[..2] != CRLF {
                return Err(RespError::InvalidProtocol(
                    "Missing CRLF after bulk string data".to_string()
                ));
            }
            buf.advance(2);

            Ok(Some(RespValue::BulkString(data.freeze())))
        } else {
            Ok(None)
        }
    }

    /// Parse array: *2\r\n$3\r\nfoo\r\n$3\r\nbar\r\n
    fn parse_array(buf: &mut BytesMut) -> Result<Option<RespValue>, RespError> {
        // Read the count line
        if let Some(line) = Self::peek_line(buf)? {
            let s = std::str::from_utf8(&line[1..])
                .map_err(|_| RespError::InvalidUtf8)?;
            let count = s.parse::<i64>()
                .map_err(|_| RespError::IntegerOverflow)?;

            if count == -1 {
                // Null array
                Self::read_line(buf)?; // consume
                return Ok(Some(RespValue::Null));
            }

            if count < 0 {
                return Err(RespError::InvalidProtocol(
                    format!("Invalid array count: {}", count)
                ));
            }

            // IMPORTANT: We need to parse elements without consuming the buffer
            // until we're sure all elements are available. Otherwise, partial
            // consumption can cause elements to be parsed as standalone values.

            // Clone the buffer to test if all elements are parseable
            let mut test_buf = buf.clone();

            // Consume the count line from test buffer
            Self::read_line(&mut test_buf)?;

            // Try to parse all elements from test buffer
            let mut elements = Vec::with_capacity(count as usize);
            for _ in 0..count {
                match Self::parse(&mut test_buf)? {
                    Some(value) => elements.push(value),
                    None => {
                        // Not enough data yet - don't consume anything from original buffer
                        return Ok(None);
                    }
                }
            }

            // All elements successfully parsed - now consume from the real buffer
            Self::read_line(buf)?; // Consume count line
            for _ in 0..count {
                // We know these will succeed since we just tested them
                Self::parse(buf)?;
            }

            Ok(Some(RespValue::Array(elements)))
        } else {
            Ok(None)
        }
    }

    /// Read a line from buffer (including CRLF), advance buffer, return without CRLF
    fn read_line(buf: &mut BytesMut) -> Result<Option<Vec<u8>>, RespError> {
        if let Some(line) = Self::peek_line(buf)? {
            buf.advance(line.len() + 2); // +2 for CRLF
            Ok(Some(line))
        } else {
            Ok(None)
        }
    }

    /// Peek a line from buffer without advancing (returns line without CRLF)
    fn peek_line(buf: &BytesMut) -> Result<Option<Vec<u8>>, RespError> {
        for i in 0..buf.len() - 1 {
            if &buf[i..i+2] == CRLF {
                return Ok(Some(buf[..i].to_vec()));
            }
        }
        Ok(None)
    }
}

/// RESP2 Encoder
pub struct RespEncoder;

impl RespEncoder {
    /// Encode a RESP value to bytes
    pub fn encode(value: &RespValue) -> Bytes {
        let mut buf = BytesMut::new();
        Self::encode_to(&mut buf, value);
        buf.freeze()
    }

    /// Encode a RESP value into an existing buffer
    pub fn encode_to(buf: &mut BytesMut, value: &RespValue) {
        match value {
            RespValue::SimpleString(s) => {
                buf.put_u8(b'+');
                buf.put_slice(s.as_bytes());
                buf.put_slice(CRLF);
            }
            RespValue::Error(e) => {
                buf.put_u8(b'-');
                buf.put_slice(e.as_bytes());
                buf.put_slice(CRLF);
            }
            RespValue::Integer(i) => {
                buf.put_u8(b':');
                buf.put_slice(i.to_string().as_bytes());
                buf.put_slice(CRLF);
            }
            RespValue::BulkString(bytes) => {
                buf.put_u8(b'$');
                buf.put_slice(bytes.len().to_string().as_bytes());
                buf.put_slice(CRLF);
                buf.put_slice(bytes);
                buf.put_slice(CRLF);
            }
            RespValue::Null => {
                buf.put_slice(b"$-1\r\n");
            }
            RespValue::Array(arr) => {
                buf.put_u8(b'*');
                buf.put_slice(arr.len().to_string().as_bytes());
                buf.put_slice(CRLF);
                for elem in arr {
                    Self::encode_to(buf, elem);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_string() {
        let mut buf = BytesMut::from("+OK\r\n");
        let result = RespParser::parse(&mut buf).unwrap();
        assert_eq!(result, Some(RespValue::SimpleString("OK".to_string())));
    }

    #[test]
    fn test_parse_error() {
        let mut buf = BytesMut::from("-Error message\r\n");
        let result = RespParser::parse(&mut buf).unwrap();
        assert_eq!(result, Some(RespValue::Error("Error message".to_string())));
    }

    #[test]
    fn test_parse_integer() {
        let mut buf = BytesMut::from(":1000\r\n");
        let result = RespParser::parse(&mut buf).unwrap();
        assert_eq!(result, Some(RespValue::Integer(1000)));
    }

    #[test]
    fn test_parse_bulk_string() {
        let mut buf = BytesMut::from("$6\r\nfoobar\r\n");
        let result = RespParser::parse(&mut buf).unwrap();
        assert_eq!(result, Some(RespValue::BulkString(Bytes::from("foobar"))));
    }

    #[test]
    fn test_parse_null() {
        let mut buf = BytesMut::from("$-1\r\n");
        let result = RespParser::parse(&mut buf).unwrap();
        assert_eq!(result, Some(RespValue::Null));
    }

    #[test]
    fn test_parse_array() {
        let mut buf = BytesMut::from("*2\r\n$3\r\nfoo\r\n$3\r\nbar\r\n");
        let result = RespParser::parse(&mut buf).unwrap();
        assert_eq!(result, Some(RespValue::Array(vec![
            RespValue::BulkString(Bytes::from("foo")),
            RespValue::BulkString(Bytes::from("bar")),
        ])));
    }

    #[test]
    fn test_encode_simple_string() {
        let value = RespValue::SimpleString("OK".to_string());
        let encoded = RespEncoder::encode(&value);
        assert_eq!(encoded, Bytes::from("+OK\r\n"));
    }

    #[test]
    fn test_encode_bulk_string() {
        let value = RespValue::BulkString(Bytes::from("foobar"));
        let encoded = RespEncoder::encode(&value);
        assert_eq!(encoded, Bytes::from("$6\r\nfoobar\r\n"));
    }
}
