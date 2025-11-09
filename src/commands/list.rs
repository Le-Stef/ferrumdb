//! List commands (LPUSH, RPUSH, LRANGE, LLEN)

use super::{Command, CommandContext, extract_bulk_string, extract_integer};
use crate::protocol::RespValue;
use crate::store::Value;
//use bytes::Bytes;

/// LPUSH command - Prepend one or multiple values to a list
///
/// Syntax: LPUSH key value [value ...]
pub struct LPushCommand;

impl Command for LPushCommand {
    fn execute(&self, ctx: &mut CommandContext, args: &[RespValue]) -> RespValue {
        if args.len() < 2 {
            return RespValue::error("ERR wrong number of arguments for 'LPUSH' command");
        }

        let key = match extract_bulk_string(&args[0]) {
            Ok(k) => k.clone(),
            Err(e) => return RespValue::error(format!("ERR {}", e)),
        };

        // Get or create list
        let list = match ctx.store.get_mut(&key) {
            Some(value) => {
                match value.as_list_mut() {
                    Some(list) => list,
                    None => return RespValue::error("WRONGTYPE Operation against a key holding the wrong kind of value"),
                }
            }
            None => {
                // Create new list
                ctx.store.set(key.clone(), Value::empty_list());
                ctx.store.get_mut(&key).unwrap().as_list_mut().unwrap()
            }
        };

        // Push all values to the front
        for i in 1..args.len() {
            let value = match extract_bulk_string(&args[i]) {
                Ok(v) => v.clone(),
                Err(e) => return RespValue::error(format!("ERR {}", e)),
            };
            list.push_front(value);
        }

        RespValue::integer(list.len() as i64)
    }

    fn name(&self) -> &'static str {
        "LPUSH"
    }

    fn min_args(&self) -> usize {
        2
    }
}

/// RPUSH command - Append one or multiple values to a list
///
/// Syntax: RPUSH key value [value ...]
pub struct RPushCommand;

impl Command for RPushCommand {
    fn execute(&self, ctx: &mut CommandContext, args: &[RespValue]) -> RespValue {
        if args.len() < 2 {
            return RespValue::error("ERR wrong number of arguments for 'RPUSH' command");
        }

        let key = match extract_bulk_string(&args[0]) {
            Ok(k) => k.clone(),
            Err(e) => return RespValue::error(format!("ERR {}", e)),
        };

        // Get or create list
        let list = match ctx.store.get_mut(&key) {
            Some(value) => {
                match value.as_list_mut() {
                    Some(list) => list,
                    None => return RespValue::error("WRONGTYPE Operation against a key holding the wrong kind of value"),
                }
            }
            None => {
                // Create new list
                ctx.store.set(key.clone(), Value::empty_list());
                ctx.store.get_mut(&key).unwrap().as_list_mut().unwrap()
            }
        };

        // Push all values to the back
        for i in 1..args.len() {
            let value = match extract_bulk_string(&args[i]) {
                Ok(v) => v.clone(),
                Err(e) => return RespValue::error(format!("ERR {}", e)),
            };
            list.push_back(value);
        }

        RespValue::integer(list.len() as i64)
    }

    fn name(&self) -> &'static str {
        "RPUSH"
    }

    fn min_args(&self) -> usize {
        2
    }
}

/// LRANGE command - Get a range of elements from a list
///
/// Syntax: LRANGE key start stop
pub struct LRangeCommand;

impl Command for LRangeCommand {
    fn execute(&self, ctx: &mut CommandContext, args: &[RespValue]) -> RespValue {
        if args.len() < 3 {
            return RespValue::error("ERR wrong number of arguments for 'LRANGE' command");
        }

        let key = match extract_bulk_string(&args[0]) {
            Ok(k) => k,
            Err(e) => return RespValue::error(format!("ERR {}", e)),
        };

        let start = match extract_integer(&args[1]) {
            Ok(i) => i,
            Err(e) => return RespValue::error(format!("ERR {}", e)),
        };

        let stop = match extract_integer(&args[2]) {
            Ok(i) => i,
            Err(e) => return RespValue::error(format!("ERR {}", e)),
        };

        // Get list
        let list = match ctx.store.get(key) {
            Some(value) => {
                match value.as_list() {
                    Some(list) => list,
                    None => return RespValue::error("WRONGTYPE Operation against a key holding the wrong kind of value"),
                }
            }
            None => {
                // Key doesn't exist, return empty array
                return RespValue::array(vec![]);
            }
        };

        let len = list.len() as i64;

        // Handle negative indices
        let start_idx = if start < 0 {
            (len + start).max(0) as usize
        } else {
            start.min(len) as usize
        };

        let stop_idx = if stop < 0 {
            (len + stop).max(-1) as usize
        } else {
            stop.min(len - 1) as usize
        };

        // Extract range
        let mut result = Vec::new();
        if start_idx <= stop_idx && start_idx < list.len() {
            for i in start_idx..=stop_idx.min(list.len() - 1) {
                if let Some(value) = list.get(i) {
                    result.push(RespValue::bulk_string(value.clone()));
                }
            }
        }

        RespValue::array(result)
    }

    fn name(&self) -> &'static str {
        "LRANGE"
    }

    fn min_args(&self) -> usize {
        3
    }

    fn max_args(&self) -> Option<usize> {
        Some(3)
    }
}

/// LLEN command - Get the length of a list
///
/// Syntax: LLEN key
pub struct LLenCommand;

impl Command for LLenCommand {
    fn execute(&self, ctx: &mut CommandContext, args: &[RespValue]) -> RespValue {
        if args.is_empty() {
            return RespValue::error("ERR wrong number of arguments for 'LLEN' command");
        }

        let key = match extract_bulk_string(&args[0]) {
            Ok(k) => k,
            Err(e) => return RespValue::error(format!("ERR {}", e)),
        };

        // Get list
        match ctx.store.get(key) {
            Some(value) => {
                match value.as_list() {
                    Some(list) => RespValue::integer(list.len() as i64),
                    None => RespValue::error("WRONGTYPE Operation against a key holding the wrong kind of value"),
                }
            }
            None => {
                // Key doesn't exist, return 0
                RespValue::integer(0)
            }
        }
    }

    fn name(&self) -> &'static str {
        "LLEN"
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

    #[test]
    fn test_lpush_rpush() {
        let mut ctx = CommandContext::new();
        let lpush_cmd = LPushCommand;
        let rpush_cmd = RPushCommand;
        let lrange_cmd = LRangeCommand;

        // RPUSH mylist a b c
        let args = vec![
            RespValue::bulk_string("mylist"),
            RespValue::bulk_string("a"),
            RespValue::bulk_string("b"),
            RespValue::bulk_string("c"),
        ];
        let result = rpush_cmd.execute(&mut ctx, &args);
        assert_eq!(result, RespValue::integer(3));

        // LPUSH mylist x
        let args = vec![
            RespValue::bulk_string("mylist"),
            RespValue::bulk_string("x"),
        ];
        let result = lpush_cmd.execute(&mut ctx, &args);
        assert_eq!(result, RespValue::integer(4));

        // LRANGE mylist 0 -1 should return [x, a, b, c]
        let args = vec![
            RespValue::bulk_string("mylist"),
            RespValue::bulk_string("0"),
            RespValue::bulk_string("-1"),
        ];
        let result = lrange_cmd.execute(&mut ctx, &args);
        let expected = RespValue::array(vec![
            RespValue::bulk_string(Bytes::from("x")),
            RespValue::bulk_string(Bytes::from("a")),
            RespValue::bulk_string(Bytes::from("b")),
            RespValue::bulk_string(Bytes::from("c")),
        ]);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_llen() {
        let mut ctx = CommandContext::new();
        let rpush_cmd = RPushCommand;
        let llen_cmd = LLenCommand;

        // RPUSH mylist a b c
        let args = vec![
            RespValue::bulk_string("mylist"),
            RespValue::bulk_string("a"),
            RespValue::bulk_string("b"),
            RespValue::bulk_string("c"),
        ];
        rpush_cmd.execute(&mut ctx, &args);

        // LLEN mylist
        let args = vec![RespValue::bulk_string("mylist")];
        let result = llen_cmd.execute(&mut ctx, &args);
        assert_eq!(result, RespValue::integer(3));

        // LLEN nonexistent
        let args = vec![RespValue::bulk_string("nonexistent")];
        let result = llen_cmd.execute(&mut ctx, &args);
        assert_eq!(result, RespValue::integer(0));
    }

    #[test]
    fn test_lrange() {
        let mut ctx = CommandContext::new();
        let rpush_cmd = RPushCommand;
        let lrange_cmd = LRangeCommand;

        // RPUSH mylist a b c d e
        let args = vec![
            RespValue::bulk_string("mylist"),
            RespValue::bulk_string("a"),
            RespValue::bulk_string("b"),
            RespValue::bulk_string("c"),
            RespValue::bulk_string("d"),
            RespValue::bulk_string("e"),
        ];
        rpush_cmd.execute(&mut ctx, &args);

        // LRANGE mylist 1 3 should return [b, c, d]
        let args = vec![
            RespValue::bulk_string("mylist"),
            RespValue::bulk_string("1"),
            RespValue::bulk_string("3"),
        ];
        let result = lrange_cmd.execute(&mut ctx, &args);
        let expected = RespValue::array(vec![
            RespValue::bulk_string(Bytes::from("b")),
            RespValue::bulk_string(Bytes::from("c")),
            RespValue::bulk_string(Bytes::from("d")),
        ]);
        assert_eq!(result, expected);

        // LRANGE mylist -2 -1 should return [d, e]
        let args = vec![
            RespValue::bulk_string("mylist"),
            RespValue::bulk_string("-2"),
            RespValue::bulk_string("-1"),
        ];
        let result = lrange_cmd.execute(&mut ctx, &args);
        let expected = RespValue::array(vec![
            RespValue::bulk_string(Bytes::from("d")),
            RespValue::bulk_string(Bytes::from("e")),
        ]);
        assert_eq!(result, expected);
    }
}
