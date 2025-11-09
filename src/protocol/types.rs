//! RESP2 value types
//!
//! Defines the core data types for RESP2 protocol

use bytes::Bytes;
use std::fmt;

/// RESP2 value types
#[derive(Debug, Clone, PartialEq)]
pub enum RespValue {
    /// Simple strings: +OK\r\n
    SimpleString(String),

    /// Errors: -Error message\r\n
    Error(String),

    /// Integers: :1000\r\n
    Integer(i64),

    /// Bulk strings: $6\r\nfoobar\r\n
    BulkString(Bytes),

    /// Null bulk string: $-1\r\n
    Null,

    /// Arrays: *2\r\n$3\r\nfoo\r\n$3\r\nbar\r\n
    Array(Vec<RespValue>),
}

impl RespValue {
    /// Create a simple string
    pub fn simple_string(s: impl Into<String>) -> Self {
        RespValue::SimpleString(s.into())
    }

    /// Create an error
    pub fn error(s: impl Into<String>) -> Self {
        RespValue::Error(s.into())
    }

    /// Create an integer
    pub fn integer(i: i64) -> Self {
        RespValue::Integer(i)
    }

    /// Create a bulk string from bytes
    pub fn bulk_string(b: impl Into<Bytes>) -> Self {
        RespValue::BulkString(b.into())
    }

    /// Create a null value
    pub fn null() -> Self {
        RespValue::Null
    }

    /// Create an array
    pub fn array(v: Vec<RespValue>) -> Self {
        RespValue::Array(v)
    }

    /// Check if this is an array
    pub fn is_array(&self) -> bool {
        matches!(self, RespValue::Array(_))
    }

    /// Try to extract array elements
    pub fn as_array(&self) -> Option<&Vec<RespValue>> {
        match self {
            RespValue::Array(arr) => Some(arr),
            _ => None,
        }
    }

    /// Try to extract bulk string bytes
    pub fn as_bulk_string(&self) -> Option<&Bytes> {
        match self {
            RespValue::BulkString(bytes) => Some(bytes),
            _ => None,
        }
    }

    /// Try to extract integer value
    pub fn as_integer(&self) -> Option<i64> {
        match self {
            RespValue::Integer(i) => Some(*i),
            _ => None,
        }
    }
}

impl fmt::Display for RespValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RespValue::SimpleString(s) => write!(f, "SimpleString({})", s),
            RespValue::Error(e) => write!(f, "Error({})", e),
            RespValue::Integer(i) => write!(f, "Integer({})", i),
            RespValue::BulkString(b) => write!(f, "BulkString({} bytes)", b.len()),
            RespValue::Null => write!(f, "Null"),
            RespValue::Array(arr) => write!(f, "Array({} elements)", arr.len()),
        }
    }
}

/// RESP parsing and encoding errors
#[derive(Debug, Clone, PartialEq)]
pub enum RespError {
    /// Incomplete data, need more bytes
    Incomplete,

    /// Invalid protocol format
    InvalidProtocol(String),

    /// Invalid UTF-8 in string
    InvalidUtf8,

    /// Integer overflow
    IntegerOverflow,

    /// IO error during parsing
    IoError(String),
}

impl fmt::Display for RespError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RespError::Incomplete => write!(f, "Incomplete data"),
            RespError::InvalidProtocol(msg) => write!(f, "Invalid protocol: {}", msg),
            RespError::InvalidUtf8 => write!(f, "Invalid UTF-8"),
            RespError::IntegerOverflow => write!(f, "Integer overflow"),
            RespError::IoError(msg) => write!(f, "IO error: {}", msg),
        }
    }
}

impl std::error::Error for RespError {}
