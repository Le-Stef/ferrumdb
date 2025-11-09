//! Search commands (KEYS, SCAN)

use super::{Command, CommandContext, extract_bulk_string};
use crate::protocol::RespValue;

/// KEYS command - Find all keys matching a pattern
///
/// Syntax: KEYS pattern
///
/// Supported patterns:
/// - * : matches all keys
/// - prefix* : matches keys starting with prefix
/// - *suffix : matches keys ending with suffix
/// - *pattern* : matches keys containing pattern
pub struct KeysCommand;

impl Command for KeysCommand {
    fn execute(&self, ctx: &mut CommandContext, args: &[RespValue]) -> RespValue {
        if args.is_empty() {
            return RespValue::error("ERR wrong number of arguments for 'KEYS' command");
        }

        let pattern = match extract_bulk_string(&args[0]) {
            Ok(p) => p,
            Err(e) => return RespValue::error(format!("ERR {}", e)),
        };

        // Convert pattern to string
        let pattern_str = match std::str::from_utf8(pattern) {
            Ok(s) => s,
            Err(_) => return RespValue::error("ERR invalid pattern encoding"),
        };

        // Get all keys from the store
        let all_keys = ctx.store.keys();

        // Filter keys based on pattern
        let matching_keys: Vec<RespValue> = all_keys
            .iter()
            .filter(|key| matches_pattern(key, pattern_str))
            .map(|key| RespValue::BulkString((*key).clone()))
            .collect();

        RespValue::Array(matching_keys)
    }

    fn name(&self) -> &'static str {
        "KEYS"
    }

    fn min_args(&self) -> usize {
        1
    }

    fn max_args(&self) -> Option<usize> {
        Some(1)
    }
}

/// Check if a key matches a pattern
///
/// Supports:
/// - * : matches everything
/// - prefix* : matches keys starting with prefix
/// - *suffix : matches keys ending with suffix
/// - *pattern* : matches keys containing pattern
fn matches_pattern(key: &[u8], pattern: &str) -> bool {
    // Convert key to string for pattern matching
    let key_str = match std::str::from_utf8(key) {
        Ok(s) => s,
        Err(_) => return false,
    };

    // Handle wildcard patterns
    if pattern == "*" {
        return true;
    }

    if pattern.starts_with('*') && pattern.ends_with('*') {
        // *pattern* : contains
        let inner = &pattern[1..pattern.len() - 1];
        return key_str.contains(inner);
    }

    if pattern.starts_with('*') {
        // *suffix : ends with
        let suffix = &pattern[1..];
        return key_str.ends_with(suffix);
    }

    if pattern.ends_with('*') {
        // prefix* : starts with
        let prefix = &pattern[..pattern.len() - 1];
        return key_str.starts_with(prefix);
    }

    // Exact match
    key_str == pattern
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::Value;

    #[test]
    fn test_keys_all() {
        let mut ctx = CommandContext::new();

        // Add some keys
        ctx.store.set("key1", Value::string("value1"));
        ctx.store.set("key2", Value::string("value2"));
        ctx.store.set("name", Value::string("Alice"));

        let cmd = KeysCommand;
        let args = vec![RespValue::bulk_string("*")];
        let result = cmd.execute(&mut ctx, &args);

        if let RespValue::Array(keys) = result {
            assert_eq!(keys.len(), 3);
        } else {
            panic!("Expected array response");
        }
    }

    #[test]
    fn test_keys_prefix() {
        let mut ctx = CommandContext::new();

        ctx.store.set("user:1", Value::string("Alice"));
        ctx.store.set("user:2", Value::string("Bob"));
        ctx.store.set("session:1", Value::string("xyz"));

        let cmd = KeysCommand;
        let args = vec![RespValue::bulk_string("user:*")];
        let result = cmd.execute(&mut ctx, &args);

        if let RespValue::Array(keys) = result {
            assert_eq!(keys.len(), 2);
        } else {
            panic!("Expected array response");
        }
    }

    #[test]
    fn test_keys_suffix() {
        let mut ctx = CommandContext::new();

        ctx.store.set("data:cache", Value::string("1"));
        ctx.store.set("temp:cache", Value::string("2"));
        ctx.store.set("data:main", Value::string("3"));

        let cmd = KeysCommand;
        let args = vec![RespValue::bulk_string("*:cache")];
        let result = cmd.execute(&mut ctx, &args);

        if let RespValue::Array(keys) = result {
            assert_eq!(keys.len(), 2);
        } else {
            panic!("Expected array response");
        }
    }

    #[test]
    fn test_keys_contains() {
        let mut ctx = CommandContext::new();

        ctx.store.set("user_admin", Value::string("1"));
        ctx.store.set("admin_role", Value::string("2"));
        ctx.store.set("role_user", Value::string("3"));

        let cmd = KeysCommand;
        let args = vec![RespValue::bulk_string("*admin*")];
        let result = cmd.execute(&mut ctx, &args);

        if let RespValue::Array(keys) = result {
            assert_eq!(keys.len(), 2);
        } else {
            panic!("Expected array response");
        }
    }

    #[test]
    fn test_keys_exact() {
        let mut ctx = CommandContext::new();

        ctx.store.set("exact_key", Value::string("1"));
        ctx.store.set("other_key", Value::string("2"));

        let cmd = KeysCommand;
        let args = vec![RespValue::bulk_string("exact_key")];
        let result = cmd.execute(&mut ctx, &args);

        if let RespValue::Array(keys) = result {
            assert_eq!(keys.len(), 1);
            if let RespValue::BulkString(key) = &keys[0] {
                assert_eq!(key, &bytes::Bytes::from("exact_key"));
            }
        } else {
            panic!("Expected array response");
        }
    }
}
