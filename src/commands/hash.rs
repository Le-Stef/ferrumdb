//! Hash commands (HSET, HGET, HGETALL, HDEL, HKEYS, HINCRBY)

use super::{Command, CommandContext, extract_bulk_string, extract_integer, log_to_aof};
use crate::protocol::RespValue;
use crate::store::Value;
use crate::aof::AofOperation;

/// HSET command - Set field in the hash stored at key to value
///
/// Syntax: HSET key field value [field value ...]
pub struct HSetCommand;

impl Command for HSetCommand {
    fn execute(&self, ctx: &mut CommandContext, args: &[RespValue]) -> RespValue {
        if args.len() < 3 {
            return RespValue::error("ERR wrong number of arguments for 'HSET' command");
        }

        // Check that we have pairs of field/value
        if (args.len() - 1) % 2 != 0 {
            return RespValue::error("ERR wrong number of arguments for 'HSET' command");
        }

        let key = match extract_bulk_string(&args[0]) {
            Ok(k) => k.clone(),
            Err(e) => return RespValue::error(format!("ERR {}", e)),
        };

        // Collect field/value pairs to set
        let mut pairs = Vec::new();
        let mut i = 1;
        while i < args.len() {
            let field = match extract_bulk_string(&args[i]) {
                Ok(f) => f.clone(),
                Err(e) => return RespValue::error(format!("ERR {}", e)),
            };

            let value = match extract_bulk_string(&args[i + 1]) {
                Ok(v) => v.clone(),
                Err(e) => return RespValue::error(format!("ERR {}", e)),
            };

            pairs.push((field, value));
            i += 2;
        }

        // Get or create hash and insert pairs
        {
            let hash = match ctx.store.get_mut(&key) {
                Some(value) => {
                    match value.as_hash_mut() {
                        Some(hash) => hash,
                        None => return RespValue::error("WRONGTYPE Operation against a key holding the wrong kind of value"),
                    }
                }
                None => {
                    // Create new hash
                    ctx.store.set(key.clone(), Value::empty_hash());
                    ctx.store.get_mut(&key).unwrap().as_hash_mut().unwrap()
                }
            };

            // Insert pairs
            let mut added = 0;
            for (field, value) in &pairs {
                if hash.insert(field.clone(), value.clone()).is_none() {
                    added += 1;
                }
            }

            // Log to AOF after releasing mutable borrow
            for (field, value) in pairs {
                log_to_aof(ctx, AofOperation::HSet, key.clone(), vec![field, value]);
            }

            return RespValue::integer(added);
        }
    }

    fn name(&self) -> &'static str {
        "HSET"
    }

    fn min_args(&self) -> usize {
        3
    }
}

/// HGET command - Get the value of a hash field
///
/// Syntax: HGET key field
pub struct HGetCommand;

impl Command for HGetCommand {
    fn execute(&self, ctx: &mut CommandContext, args: &[RespValue]) -> RespValue {
        if args.len() < 2 {
            return RespValue::error("ERR wrong number of arguments for 'HGET' command");
        }

        let key = match extract_bulk_string(&args[0]) {
            Ok(k) => k,
            Err(e) => return RespValue::error(format!("ERR {}", e)),
        };

        let field = match extract_bulk_string(&args[1]) {
            Ok(f) => f,
            Err(e) => return RespValue::error(format!("ERR {}", e)),
        };

        // Get hash
        match ctx.store.get(key) {
            Some(value) => {
                match value.as_hash() {
                    Some(hash) => {
                        match hash.get(field) {
                            Some(v) => RespValue::bulk_string(v.clone()),
                            None => RespValue::null(),
                        }
                    }
                    None => RespValue::error("WRONGTYPE Operation against a key holding the wrong kind of value"),
                }
            }
            None => RespValue::null(),
        }
    }

    fn name(&self) -> &'static str {
        "HGET"
    }

    fn min_args(&self) -> usize {
        2
    }

    fn max_args(&self) -> Option<usize> {
        Some(2)
    }
}

/// HGETALL command - Get all fields and values in a hash
///
/// Syntax: HGETALL key
pub struct HGetAllCommand;

impl Command for HGetAllCommand {
    fn execute(&self, ctx: &mut CommandContext, args: &[RespValue]) -> RespValue {
        if args.is_empty() {
            return RespValue::error("ERR wrong number of arguments for 'HGETALL' command");
        }

        let key = match extract_bulk_string(&args[0]) {
            Ok(k) => k,
            Err(e) => return RespValue::error(format!("ERR {}", e)),
        };

        // Get hash
        match ctx.store.get(key) {
            Some(value) => {
                match value.as_hash() {
                    Some(hash) => {
                        let mut result = Vec::new();
                        for (field, value) in hash.iter() {
                            result.push(RespValue::bulk_string(field.clone()));
                            result.push(RespValue::bulk_string(value.clone()));
                        }
                        RespValue::array(result)
                    }
                    None => RespValue::error("WRONGTYPE Operation against a key holding the wrong kind of value"),
                }
            }
            None => RespValue::array(vec![]),
        }
    }

    fn name(&self) -> &'static str {
        "HGETALL"
    }

    fn min_args(&self) -> usize {
        1
    }

    fn max_args(&self) -> Option<usize> {
        Some(1)
    }
}

/// HDEL command - Delete one or more hash fields
///
/// Syntax: HDEL key field [field ...]
pub struct HDelCommand;

impl Command for HDelCommand {
    fn execute(&self, ctx: &mut CommandContext, args: &[RespValue]) -> RespValue {
        if args.len() < 2 {
            return RespValue::error("ERR wrong number of arguments for 'HDEL' command");
        }

        let key = match extract_bulk_string(&args[0]) {
            Ok(k) => k,
            Err(e) => return RespValue::error(format!("ERR {}", e)),
        };

        // Get hash and delete fields
        let mut deleted_fields = Vec::new();
        {
            let hash = match ctx.store.get_mut(&key) {
                Some(value) => {
                    match value.as_hash_mut() {
                        Some(hash) => hash,
                        None => return RespValue::error("WRONGTYPE Operation against a key holding the wrong kind of value"),
                    }
                }
                None => return RespValue::integer(0),
            };

            // Delete all fields
            for i in 1..args.len() {
                let field = match extract_bulk_string(&args[i]) {
                    Ok(f) => f,
                    Err(e) => return RespValue::error(format!("ERR {}", e)),
                };

                if hash.remove(field).is_some() {
                    deleted_fields.push(field.clone());
                }
            }
        }

        // Log to AOF after releasing mutable borrow
        for field in &deleted_fields {
            log_to_aof(ctx, AofOperation::HDel, key.clone(), vec![field.clone()]);
        }

        RespValue::integer(deleted_fields.len() as i64)
    }

    fn name(&self) -> &'static str {
        "HDEL"
    }

    fn min_args(&self) -> usize {
        2
    }
}

/// HKEYS command - Get all field names in a hash
///
/// Syntax: HKEYS key
pub struct HKeysCommand;

impl Command for HKeysCommand {
    fn execute(&self, ctx: &mut CommandContext, args: &[RespValue]) -> RespValue {
        if args.is_empty() {
            return RespValue::error("ERR wrong number of arguments for 'HKEYS' command");
        }

        let key = match extract_bulk_string(&args[0]) {
            Ok(k) => k,
            Err(e) => return RespValue::error(format!("ERR {}", e)),
        };

        // Get hash
        match ctx.store.get(key) {
            Some(value) => {
                match value.as_hash() {
                    Some(hash) => {
                        let keys: Vec<RespValue> = hash
                            .keys()
                            .map(|k| RespValue::bulk_string(k.clone()))
                            .collect();
                        RespValue::array(keys)
                    }
                    None => RespValue::error("WRONGTYPE Operation against a key holding the wrong kind of value"),
                }
            }
            None => RespValue::array(vec![]),
        }
    }

    fn name(&self) -> &'static str {
        "HKEYS"
    }

    fn min_args(&self) -> usize {
        1
    }

    fn max_args(&self) -> Option<usize> {
        Some(1)
    }
}

/// HINCRBY command - Increment the integer value of a hash field by the given number
///
/// Syntax: HINCRBY key field increment
pub struct HIncrByCommand;

impl Command for HIncrByCommand {
    fn execute(&self, ctx: &mut CommandContext, args: &[RespValue]) -> RespValue {
        if args.len() < 3 {
            return RespValue::error("ERR wrong number of arguments for 'HINCRBY' command");
        }

        let key = match extract_bulk_string(&args[0]) {
            Ok(k) => k.clone(),
            Err(e) => return RespValue::error(format!("ERR {}", e)),
        };

        let field = match extract_bulk_string(&args[1]) {
            Ok(f) => f.clone(),
            Err(e) => return RespValue::error(format!("ERR {}", e)),
        };

        let increment = match extract_integer(&args[2]) {
            Ok(i) => i,
            Err(e) => return RespValue::error(format!("ERR {}", e)),
        };

        // Get or create hash
        let hash = match ctx.store.get_mut(&key) {
            Some(value) => {
                match value.as_hash_mut() {
                    Some(hash) => hash,
                    None => return RespValue::error("WRONGTYPE Operation against a key holding the wrong kind of value"),
                }
            }
            None => {
                // Create new hash
                ctx.store.set(key.clone(), Value::empty_hash());
                ctx.store.get_mut(&key).unwrap().as_hash_mut().unwrap()
            }
        };

        // Get current value or initialize to 0
        let current = match hash.get(&field) {
            Some(bytes) => {
                let s = match std::str::from_utf8(bytes) {
                    Ok(s) => s,
                    Err(_) => return RespValue::error("ERR hash value is not an integer"),
                };
                match s.parse::<i64>() {
                    Ok(i) => i,
                    Err(_) => return RespValue::error("ERR hash value is not an integer"),
                }
            }
            None => 0,
        };

        // Increment
        let new_value = match current.checked_add(increment) {
            Some(v) => v,
            None => return RespValue::error("ERR increment would overflow"),
        };

        // Store new value as string
        hash.insert(field.clone(), new_value.to_string().into());

        // Log to AOF
        use bytes::Bytes;
        log_to_aof(
            ctx,
            AofOperation::HSet,  // We use HSet for HINCRBY replay
            key.clone(),
            vec![field, Bytes::from(new_value.to_string())],
        );

        RespValue::integer(new_value)
    }

    fn name(&self) -> &'static str {
        "HINCRBY"
    }

    fn min_args(&self) -> usize {
        3
    }

    fn max_args(&self) -> Option<usize> {
        Some(3)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;

    #[test]
    fn test_hset_hget() {
        let mut ctx = CommandContext::new();
        let hset_cmd = HSetCommand;
        let hget_cmd = HGetCommand;

        // HSET myhash field1 value1
        let args = vec![
            RespValue::bulk_string("myhash"),
            RespValue::bulk_string("field1"),
            RespValue::bulk_string("value1"),
        ];
        let result = hset_cmd.execute(&mut ctx, &args);
        assert_eq!(result, RespValue::integer(1));

        // HGET myhash field1
        let args = vec![
            RespValue::bulk_string("myhash"),
            RespValue::bulk_string("field1"),
        ];
        let result = hget_cmd.execute(&mut ctx, &args);
        assert_eq!(result, RespValue::bulk_string(Bytes::from("value1")));

        // HGET myhash nonexistent
        let args = vec![
            RespValue::bulk_string("myhash"),
            RespValue::bulk_string("nonexistent"),
        ];
        let result = hget_cmd.execute(&mut ctx, &args);
        assert_eq!(result, RespValue::null());
    }

    #[test]
    fn test_hgetall() {
        let mut ctx = CommandContext::new();
        let hset_cmd = HSetCommand;
        let hgetall_cmd = HGetAllCommand;

        // HSET myhash field1 value1 field2 value2
        let args = vec![
            RespValue::bulk_string("myhash"),
            RespValue::bulk_string("field1"),
            RespValue::bulk_string("value1"),
            RespValue::bulk_string("field2"),
            RespValue::bulk_string("value2"),
        ];
        hset_cmd.execute(&mut ctx, &args);

        // HGETALL myhash
        let args = vec![RespValue::bulk_string("myhash")];
        let result = hgetall_cmd.execute(&mut ctx, &args);

        // Should return array with 4 elements (2 fields + 2 values)
        if let RespValue::Array(arr) = result {
            assert_eq!(arr.len(), 4);
        } else {
            panic!("Expected array response");
        }
    }

    #[test]
    fn test_hdel() {
        let mut ctx = CommandContext::new();
        let hset_cmd = HSetCommand;
        let hdel_cmd = HDelCommand;

        // HSET myhash field1 value1 field2 value2
        let args = vec![
            RespValue::bulk_string("myhash"),
            RespValue::bulk_string("field1"),
            RespValue::bulk_string("value1"),
            RespValue::bulk_string("field2"),
            RespValue::bulk_string("value2"),
        ];
        hset_cmd.execute(&mut ctx, &args);

        // HDEL myhash field1
        let args = vec![
            RespValue::bulk_string("myhash"),
            RespValue::bulk_string("field1"),
        ];
        let result = hdel_cmd.execute(&mut ctx, &args);
        assert_eq!(result, RespValue::integer(1));

        // HDEL myhash field1 again (should return 0)
        let result = hdel_cmd.execute(&mut ctx, &args);
        assert_eq!(result, RespValue::integer(0));
    }

    #[test]
    fn test_hincrby() {
        let mut ctx = CommandContext::new();
        let hincrby_cmd = HIncrByCommand;

        // HINCRBY myhash counter 10
        let args = vec![
            RespValue::bulk_string("myhash"),
            RespValue::bulk_string("counter"),
            RespValue::bulk_string("10"),
        ];
        let result = hincrby_cmd.execute(&mut ctx, &args);
        assert_eq!(result, RespValue::integer(10));

        // HINCRBY myhash counter 5
        let args = vec![
            RespValue::bulk_string("myhash"),
            RespValue::bulk_string("counter"),
            RespValue::bulk_string("5"),
        ];
        let result = hincrby_cmd.execute(&mut ctx, &args);
        assert_eq!(result, RespValue::integer(15));
    }
}
