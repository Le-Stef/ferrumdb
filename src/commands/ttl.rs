//! TTL commands (EXPIRE, TTL)

use super::{Command, CommandContext, extract_bulk_string, extract_integer, log_to_aof};
use crate::protocol::RespValue;
use crate::aof::AofOperation;
use bytes::Bytes;

/// EXPIRE command - Set a timeout on a key
///
/// Syntax: EXPIRE key seconds
pub struct ExpireCommand;

impl Command for ExpireCommand {
    fn execute(&self, ctx: &mut CommandContext, args: &[RespValue]) -> RespValue {
        // Validate argument count
        if args.len() < 2 {
            return RespValue::error("ERR wrong number of arguments for 'EXPIRE' command");
        }

        // Extract key
        let key = match extract_bulk_string(&args[0]) {
            Ok(k) => k,
            Err(e) => return RespValue::error(format!("ERR {}", e)),
        };

        // Extract TTL seconds
        let seconds = match extract_integer(&args[1]) {
            Ok(s) => s,
            Err(e) => return RespValue::error(format!("ERR {}", e)),
        };

        // Set expiration
        if ctx.store.expire(key, seconds) {
            // Log to AOF
            log_to_aof(
                ctx,
                AofOperation::Expire,
                key.clone(),
                vec![Bytes::from(seconds.to_string())],
            );
            RespValue::integer(1)
        } else {
            RespValue::integer(0)
        }
    }

    fn name(&self) -> &'static str {
        "EXPIRE"
    }

    fn min_args(&self) -> usize {
        2
    }

    fn max_args(&self) -> Option<usize> {
        Some(2)
    }
}

/// TTL command - Get the time to live for a key
///
/// Syntax: TTL key
///
/// Returns:
/// - The TTL in seconds
/// - -1 if the key exists but has no expiration
/// - -2 if the key does not exist
pub struct TtlCommand;

impl Command for TtlCommand {
    fn execute(&self, ctx: &mut CommandContext, args: &[RespValue]) -> RespValue {
        // Validate argument count
        if args.is_empty() {
            return RespValue::error("ERR wrong number of arguments for 'TTL' command");
        }

        // Extract key
        let key = match extract_bulk_string(&args[0]) {
            Ok(k) => k,
            Err(e) => return RespValue::error(format!("ERR {}", e)),
        };

        // Get TTL
        let ttl = ctx.store.ttl(key);
        RespValue::integer(ttl)
    }

    fn name(&self) -> &'static str {
        "TTL"
    }

    fn min_args(&self) -> usize {
        1
    }

    fn max_args(&self) -> Option<usize> {
        Some(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::Value;

    #[test]
    fn test_expire_ttl() {
        let mut ctx = CommandContext::new();
        ctx.store.set("key1", Value::string("value1"));

        let expire_cmd = ExpireCommand;
        let ttl_cmd = TtlCommand;

        // Set expiration to 100 seconds
        let args = vec![
            RespValue::bulk_string("key1"),
            RespValue::bulk_string("100"),
        ];
        let result = expire_cmd.execute(&mut ctx, &args);
        assert_eq!(result, RespValue::integer(1));

        // Check TTL (should be around 100, but might be 99 due to timing)
        let args = vec![RespValue::bulk_string("key1")];
        let result = ttl_cmd.execute(&mut ctx, &args);
        if let RespValue::Integer(ttl) = result {
            assert!(ttl >= 99 && ttl <= 100);
        } else {
            panic!("Expected integer response");
        }
    }

    #[test]
    fn test_ttl_no_key() {
        let mut ctx = CommandContext::new();
        let ttl_cmd = TtlCommand;

        let args = vec![RespValue::bulk_string("nonexistent")];
        let result = ttl_cmd.execute(&mut ctx, &args);
        assert_eq!(result, RespValue::integer(-2));
    }

    #[test]
    fn test_ttl_no_expiration() {
        let mut ctx = CommandContext::new();
        ctx.store.set("key1", Value::string("value1"));

        let ttl_cmd = TtlCommand;

        let args = vec![RespValue::bulk_string("key1")];
        let result = ttl_cmd.execute(&mut ctx, &args);
        assert_eq!(result, RespValue::integer(-1));
    }
}
