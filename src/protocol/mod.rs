//! RESP2 protocol implementation
//!
//! This module handles parsing and encoding of Redis Serialization Protocol (RESP2).
//! It is completely independent from other modules (loose coupling).

mod types;
mod resp;

pub use types::{RespValue, RespError};
pub use resp::{RespParser, RespEncoder};
