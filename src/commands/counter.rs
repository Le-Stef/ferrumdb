//! Counter commands (INCR, INCRBY, DECR, DECRBY)

use super::{Command, CommandContext, extract_bulk_string, extract_integer, log_to_aof};
use crate::protocol::RespValue;
use crate::store::Value;
use crate::aof::AofOperation;
use bytes::Bytes;

/// INCR command - Increment the integer value of a key by 1
///
/// Syntax: INCR key
pub struct IncrCommand;

impl Command for IncrCommand {
    fn execute(&self, ctx: &mut CommandContext, args: &[RespValue]) -> RespValue {
        if args.is_empty() {
            return RespValue::error("ERR wrong number of arguments for 'INCR' command");
        }

        let key = match extract_bulk_string(&args[0]) {
            Ok(k) => k.clone(),
            Err(e) => return RespValue::error(format!("ERR {}", e)),
        };

        // Get current value or initialize to 0
        let new_value = match ctx.store.get_mut(&key) {
            Some(value) => {
                match value {
                    Value::Integer(ref mut i) => {
                        *i = match i.checked_add(1) {
                            Some(v) => v,
                            None => return RespValue::error("ERR increment would overflow"),
                        };
                        *i
                    }
                    Value::String(bytes) => {
                        // Try to parse as integer
                        let s = match std::str::from_utf8(bytes) {
                            Ok(s) => s,
                            Err(_) => return RespValue::error("ERR value is not an integer or out of range"),
                        };
                        let mut i = match s.parse::<i64>() {
                            Ok(i) => i,
                            Err(_) => return RespValue::error("ERR value is not an integer or out of range"),
                        };
                        i = match i.checked_add(1) {
                            Some(v) => v,
                            None => return RespValue::error("ERR increment would overflow"),
                        };
                        *value = Value::Integer(i);
                        i
                    }
                    _ => return RespValue::error("WRONGTYPE Operation against a key holding the wrong kind of value"),
                }
            }
            None => {
                // Key doesn't exist, initialize to 1
                ctx.store.set(key.clone(), Value::Integer(1));
                1
            }
        };

        // Log to AOF
        log_to_aof(ctx, AofOperation::Set, key, vec![Bytes::from(new_value.to_string())]);

        RespValue::integer(new_value)
    }

    fn name(&self) -> &'static str {
        "INCR"
    }

    fn min_args(&self) -> usize {
        1
    }

    fn max_args(&self) -> Option<usize> {
        Some(1)
    }
}

/// INCRBY command - Increment the integer value of a key by the given amount
///
/// Syntax: INCRBY key increment
pub struct IncrByCommand;

impl Command for IncrByCommand {
    fn execute(&self, ctx: &mut CommandContext, args: &[RespValue]) -> RespValue {
        if args.len() < 2 {
            return RespValue::error("ERR wrong number of arguments for 'INCRBY' command");
        }

        let key = match extract_bulk_string(&args[0]) {
            Ok(k) => k.clone(),
            Err(e) => return RespValue::error(format!("ERR {}", e)),
        };

        let increment = match extract_integer(&args[1]) {
            Ok(i) => i,
            Err(e) => return RespValue::error(format!("ERR {}", e)),
        };

        // Get current value or initialize to 0
        let new_value = match ctx.store.get_mut(&key) {
            Some(value) => {
                match value {
                    Value::Integer(ref mut i) => {
                        *i = match i.checked_add(increment) {
                            Some(v) => v,
                            None => return RespValue::error("ERR increment would overflow"),
                        };
                        *i
                    }
                    Value::String(bytes) => {
                        // Try to parse as integer
                        let s = match std::str::from_utf8(bytes) {
                            Ok(s) => s,
                            Err(_) => return RespValue::error("ERR value is not an integer or out of range"),
                        };
                        let mut i = match s.parse::<i64>() {
                            Ok(i) => i,
                            Err(_) => return RespValue::error("ERR value is not an integer or out of range"),
                        };
                        i = match i.checked_add(increment) {
                            Some(v) => v,
                            None => return RespValue::error("ERR increment would overflow"),
                        };
                        *value = Value::Integer(i);
                        i
                    }
                    _ => return RespValue::error("WRONGTYPE Operation against a key holding the wrong kind of value"),
                }
            }
            None => {
                // Key doesn't exist, initialize to increment
                ctx.store.set(key, Value::Integer(increment));
                increment
            }
        };

        RespValue::integer(new_value)
    }

    fn name(&self) -> &'static str {
        "INCRBY"
    }

    fn min_args(&self) -> usize {
        2
    }

    fn max_args(&self) -> Option<usize> {
        Some(2)
    }
}

/// DECR command - Decrement the integer value of a key by 1
///
/// Syntax: DECR key
pub struct DecrCommand;

impl Command for DecrCommand {
    fn execute(&self, ctx: &mut CommandContext, args: &[RespValue]) -> RespValue {
        if args.is_empty() {
            return RespValue::error("ERR wrong number of arguments for 'DECR' command");
        }

        let key = match extract_bulk_string(&args[0]) {
            Ok(k) => k.clone(),
            Err(e) => return RespValue::error(format!("ERR {}", e)),
        };

        // Get current value or initialize to 0
        let new_value = match ctx.store.get_mut(&key) {
            Some(value) => {
                match value {
                    Value::Integer(ref mut i) => {
                        *i = match i.checked_sub(1) {
                            Some(v) => v,
                            None => return RespValue::error("ERR decrement would overflow"),
                        };
                        *i
                    }
                    Value::String(bytes) => {
                        // Try to parse as integer
                        let s = match std::str::from_utf8(bytes) {
                            Ok(s) => s,
                            Err(_) => return RespValue::error("ERR value is not an integer or out of range"),
                        };
                        let mut i = match s.parse::<i64>() {
                            Ok(i) => i,
                            Err(_) => return RespValue::error("ERR value is not an integer or out of range"),
                        };
                        i = match i.checked_sub(1) {
                            Some(v) => v,
                            None => return RespValue::error("ERR decrement would overflow"),
                        };
                        *value = Value::Integer(i);
                        i
                    }
                    _ => return RespValue::error("WRONGTYPE Operation against a key holding the wrong kind of value"),
                }
            }
            None => {
                // Key doesn't exist, initialize to -1
                ctx.store.set(key, Value::Integer(-1));
                -1
            }
        };

        RespValue::integer(new_value)
    }

    fn name(&self) -> &'static str {
        "DECR"
    }

    fn min_args(&self) -> usize {
        1
    }

    fn max_args(&self) -> Option<usize> {
        Some(1)
    }
}

/// DECRBY command - Decrement the integer value of a key by the given amount
///
/// Syntax: DECRBY key decrement
pub struct DecrByCommand;

impl Command for DecrByCommand {
    fn execute(&self, ctx: &mut CommandContext, args: &[RespValue]) -> RespValue {
        if args.len() < 2 {
            return RespValue::error("ERR wrong number of arguments for 'DECRBY' command");
        }

        let key = match extract_bulk_string(&args[0]) {
            Ok(k) => k.clone(),
            Err(e) => return RespValue::error(format!("ERR {}", e)),
        };

        let decrement = match extract_integer(&args[1]) {
            Ok(i) => i,
            Err(e) => return RespValue::error(format!("ERR {}", e)),
        };

        // Get current value or initialize to 0
        let new_value = match ctx.store.get_mut(&key) {
            Some(value) => {
                match value {
                    Value::Integer(ref mut i) => {
                        *i = match i.checked_sub(decrement) {
                            Some(v) => v,
                            None => return RespValue::error("ERR decrement would overflow"),
                        };
                        *i
                    }
                    Value::String(bytes) => {
                        // Try to parse as integer
                        let s = match std::str::from_utf8(bytes) {
                            Ok(s) => s,
                            Err(_) => return RespValue::error("ERR value is not an integer or out of range"),
                        };
                        let mut i = match s.parse::<i64>() {
                            Ok(i) => i,
                            Err(_) => return RespValue::error("ERR value is not an integer or out of range"),
                        };
                        i = match i.checked_sub(decrement) {
                            Some(v) => v,
                            None => return RespValue::error("ERR decrement would overflow"),
                        };
                        *value = Value::Integer(i);
                        i
                    }
                    _ => return RespValue::error("WRONGTYPE Operation against a key holding the wrong kind of value"),
                }
            }
            None => {
                // Key doesn't exist, initialize to -decrement
                ctx.store.set(key, Value::Integer(-decrement));
                -decrement
            }
        };

        RespValue::integer(new_value)
    }

    fn name(&self) -> &'static str {
        "DECRBY"
    }

    fn min_args(&self) -> usize {
        2
    }

    fn max_args(&self) -> Option<usize> {
        Some(2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_incr() {
        let mut ctx = CommandContext::new();
        let cmd = IncrCommand;

        // INCR on non-existent key
        let args = vec![RespValue::bulk_string("counter")];
        let result = cmd.execute(&mut ctx, &args);
        assert_eq!(result, RespValue::integer(1));

        // INCR again
        let result = cmd.execute(&mut ctx, &args);
        assert_eq!(result, RespValue::integer(2));
    }

    #[test]
    fn test_incrby() {
        let mut ctx = CommandContext::new();
        let cmd = IncrByCommand;

        // INCRBY on non-existent key
        let args = vec![
            RespValue::bulk_string("counter"),
            RespValue::bulk_string("10"),
        ];
        let result = cmd.execute(&mut ctx, &args);
        assert_eq!(result, RespValue::integer(10));

        // INCRBY again
        let result = cmd.execute(&mut ctx, &args);
        assert_eq!(result, RespValue::integer(20));
    }

    #[test]
    fn test_decr() {
        let mut ctx = CommandContext::new();
        let cmd = DecrCommand;

        // DECR on non-existent key
        let args = vec![RespValue::bulk_string("counter")];
        let result = cmd.execute(&mut ctx, &args);
        assert_eq!(result, RespValue::integer(-1));

        // DECR again
        let result = cmd.execute(&mut ctx, &args);
        assert_eq!(result, RespValue::integer(-2));
    }

    #[test]
    fn test_decrby() {
        let mut ctx = CommandContext::new();
        let cmd = DecrByCommand;

        // DECRBY on non-existent key
        let args = vec![
            RespValue::bulk_string("counter"),
            RespValue::bulk_string("5"),
        ];
        let result = cmd.execute(&mut ctx, &args);
        assert_eq!(result, RespValue::integer(-5));

        // DECRBY again
        let result = cmd.execute(&mut ctx, &args);
        assert_eq!(result, RespValue::integer(-10));
    }
}
