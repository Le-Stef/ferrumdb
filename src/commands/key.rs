//! Key commands (DEL, EXISTS)

use super::{Command, CommandContext, extract_bulk_string, log_to_aof};
use crate::protocol::RespValue;
use crate::aof::AofOperation;

/// DEL command - Delete one or more keys
///
/// Syntax: DEL key [key ...]
pub struct DelCommand;

impl Command for DelCommand {
    fn execute(&self, ctx: &mut CommandContext, args: &[RespValue]) -> RespValue {
        // Validate argument count
        if args.is_empty() {
            return RespValue::error("ERR wrong number of arguments for 'DEL' command");
        }

        let mut deleted = 0;

        // Delete each key
        for arg in args {
            let key = match extract_bulk_string(arg) {
                Ok(k) => k,
                Err(e) => return RespValue::error(format!("ERR {}", e)),
            };

            if ctx.store.delete(key) {
                // Log to AOF after successful deletion
                log_to_aof(ctx, AofOperation::Del, key.clone(), vec![]);
                deleted += 1;
            }
        }

        RespValue::integer(deleted)
    }

    fn name(&self) -> &'static str {
        "DEL"
    }

    fn min_args(&self) -> usize {
        1
    }
}

/// EXISTS command - Check if one or more keys exist
///
/// Syntax: EXISTS key [key ...]
pub struct ExistsCommand;

impl Command for ExistsCommand {
    fn execute(&self, ctx: &mut CommandContext, args: &[RespValue]) -> RespValue {
        // Validate argument count
        if args.is_empty() {
            return RespValue::error("ERR wrong number of arguments for 'EXISTS' command");
        }

        let mut count = 0;

        // Check each key
        for arg in args {
            let key = match extract_bulk_string(arg) {
                Ok(k) => k,
                Err(e) => return RespValue::error(format!("ERR {}", e)),
            };

            if ctx.store.exists(key) {
                count += 1;
            }
        }

        RespValue::integer(count)
    }

    fn name(&self) -> &'static str {
        "EXISTS"
    }

    fn min_args(&self) -> usize {
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::Value;

    #[test]
    fn test_del() {
        let mut ctx = CommandContext::new();
        ctx.store.set("key1", Value::string("value1"));
        ctx.store.set("key2", Value::string("value2"));

        let del_cmd = DelCommand;

        let args = vec![
            RespValue::bulk_string("key1"),
            RespValue::bulk_string("key2"),
            RespValue::bulk_string("key3"), // doesn't exist
        ];

        let result = del_cmd.execute(&mut ctx, &args);
        assert_eq!(result, RespValue::integer(2));
    }

    #[test]
    fn test_exists() {
        let mut ctx = CommandContext::new();
        ctx.store.set("key1", Value::string("value1"));

        let exists_cmd = ExistsCommand;

        let args = vec![
            RespValue::bulk_string("key1"),
            RespValue::bulk_string("key2"), // doesn't exist
        ];

        let result = exists_cmd.execute(&mut ctx, &args);
        assert_eq!(result, RespValue::integer(1));
    }
}
