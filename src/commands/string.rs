//! String commands (SET, GET)

use super::{Command, CommandContext, extract_bulk_string, log_to_aof};
use crate::protocol::RespValue;
use crate::store::Value;
use crate::aof::AofOperation;

/// SET command - Set a key to a value
///
/// Syntax: SET key value
pub struct SetCommand;

impl Command for SetCommand {
    fn execute(&self, ctx: &mut CommandContext, args: &[RespValue]) -> RespValue {
        // Validate argument count
        if args.len() < 2 {
            return RespValue::error("ERR wrong number of arguments for 'SET' command");
        }

        // Extract key and value
        let key = match extract_bulk_string(&args[0]) {
            Ok(k) => k.clone(),
            Err(e) => return RespValue::error(format!("ERR {}", e)),
        };

        let value = match extract_bulk_string(&args[1]) {
            Ok(v) => v.clone(),
            Err(e) => return RespValue::error(format!("ERR {}", e)),
        };

        // TODO: Parse optional arguments (EX, PX, NX, XX, etc.) in future phases

        // Log to AOF
        log_to_aof(ctx, AofOperation::Set, key.clone(), vec![value.clone()]);

        // Set the value
        ctx.store.set(key, Value::String(value));

        RespValue::simple_string("OK")
    }

    fn name(&self) -> &'static str {
        "SET"
    }

    fn min_args(&self) -> usize {
        2
    }
}

/// GET command - Get the value of a key
///
/// Syntax: GET key
pub struct GetCommand;

impl Command for GetCommand {
    fn execute(&self, ctx: &mut CommandContext, args: &[RespValue]) -> RespValue {
        // Validate argument count
        if args.is_empty() {
            return RespValue::error("ERR wrong number of arguments for 'GET' command");
        }

        // Extract key
        let key = match extract_bulk_string(&args[0]) {
            Ok(k) => k,
            Err(e) => return RespValue::error(format!("ERR {}", e)),
        };

        // Get the value
        match ctx.store.get(key) {
            Some(value) => {
                match value {
                    Value::String(bytes) => RespValue::bulk_string(bytes.clone()),
                    Value::Integer(i) => {
                        // Convert integer to string
                        RespValue::bulk_string(i.to_string())
                    }
                    _ => RespValue::error("WRONGTYPE Operation against a key holding the wrong kind of value"),
                }
            }
            None => RespValue::null(),
        }
    }

    fn name(&self) -> &'static str {
        "GET"
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
    use bytes::Bytes;

    #[test]
    fn test_set_get() {
        let mut ctx = CommandContext::new();

        let set_cmd = SetCommand;
        let get_cmd = GetCommand;

        let args = vec![
            RespValue::bulk_string("mykey"),
            RespValue::bulk_string("myvalue"),
        ];

        let result = set_cmd.execute(&mut ctx, &args);
        assert_eq!(result, RespValue::simple_string("OK"));

        let args = vec![RespValue::bulk_string("mykey")];
        let result = get_cmd.execute(&mut ctx, &args);
        assert_eq!(result, RespValue::bulk_string(Bytes::from("myvalue")));
    }

    #[test]
    fn test_get_nonexistent() {
        let mut ctx = CommandContext::new();
        let get_cmd = GetCommand;

        let args = vec![RespValue::bulk_string("nonexistent")];
        let result = get_cmd.execute(&mut ctx, &args);
        assert_eq!(result, RespValue::null());
    }
}
