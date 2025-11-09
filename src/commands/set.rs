//! Set commands (SADD, SMEMBERS, SCARD)

use super::{Command, CommandContext, extract_bulk_string};
use crate::protocol::RespValue;
use crate::store::Value;

/// SADD command - Add one or more members to a set
///
/// Syntax: SADD key member [member ...]
pub struct SAddCommand;

impl Command for SAddCommand {
    fn execute(&self, ctx: &mut CommandContext, args: &[RespValue]) -> RespValue {
        if args.len() < 2 {
            return RespValue::error("ERR wrong number of arguments for 'SADD' command");
        }

        let key = match extract_bulk_string(&args[0]) {
            Ok(k) => k.clone(),
            Err(e) => return RespValue::error(format!("ERR {}", e)),
        };

        // Get or create set
        let set = match ctx.store.get_mut(&key) {
            Some(value) => {
                match value.as_set_mut() {
                    Some(set) => set,
                    None => return RespValue::error("WRONGTYPE Operation against a key holding the wrong kind of value"),
                }
            }
            None => {
                // Create new set
                ctx.store.set(key.clone(), Value::empty_set());
                ctx.store.get_mut(&key).unwrap().as_set_mut().unwrap()
            }
        };

        // Add all members
        let mut added = 0;
        for i in 1..args.len() {
            let member = match extract_bulk_string(&args[i]) {
                Ok(m) => m.clone(),
                Err(e) => return RespValue::error(format!("ERR {}", e)),
            };

            if set.insert(member) {
                added += 1;
            }
        }

        RespValue::integer(added)
    }

    fn name(&self) -> &'static str {
        "SADD"
    }

    fn min_args(&self) -> usize {
        2
    }
}

/// SMEMBERS command - Get all members of a set
///
/// Syntax: SMEMBERS key
pub struct SMembersCommand;

impl Command for SMembersCommand {
    fn execute(&self, ctx: &mut CommandContext, args: &[RespValue]) -> RespValue {
        if args.is_empty() {
            return RespValue::error("ERR wrong number of arguments for 'SMEMBERS' command");
        }

        let key = match extract_bulk_string(&args[0]) {
            Ok(k) => k,
            Err(e) => return RespValue::error(format!("ERR {}", e)),
        };

        // Get set
        match ctx.store.get(key) {
            Some(value) => {
                match value.as_set() {
                    Some(set) => {
                        let members: Vec<RespValue> = set
                            .iter()
                            .map(|m| RespValue::bulk_string(m.clone()))
                            .collect();
                        RespValue::array(members)
                    }
                    None => RespValue::error("WRONGTYPE Operation against a key holding the wrong kind of value"),
                }
            }
            None => {
                // Key doesn't exist, return empty array
                RespValue::array(vec![])
            }
        }
    }

    fn name(&self) -> &'static str {
        "SMEMBERS"
    }

    fn min_args(&self) -> usize {
        1
    }

    fn max_args(&self) -> Option<usize> {
        Some(1)
    }
}

/// SCARD command - Get the number of members in a set
///
/// Syntax: SCARD key
pub struct SCardCommand;

impl Command for SCardCommand {
    fn execute(&self, ctx: &mut CommandContext, args: &[RespValue]) -> RespValue {
        if args.is_empty() {
            return RespValue::error("ERR wrong number of arguments for 'SCARD' command");
        }

        let key = match extract_bulk_string(&args[0]) {
            Ok(k) => k,
            Err(e) => return RespValue::error(format!("ERR {}", e)),
        };

        // Get set
        match ctx.store.get(key) {
            Some(value) => {
                match value.as_set() {
                    Some(set) => RespValue::integer(set.len() as i64),
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
        "SCARD"
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
    fn test_sadd() {
        let mut ctx = CommandContext::new();
        let cmd = SAddCommand;

        // SADD myset a b c
        let args = vec![
            RespValue::bulk_string("myset"),
            RespValue::bulk_string("a"),
            RespValue::bulk_string("b"),
            RespValue::bulk_string("c"),
        ];
        let result = cmd.execute(&mut ctx, &args);
        assert_eq!(result, RespValue::integer(3));

        // SADD myset b c d (b and c already exist)
        let args = vec![
            RespValue::bulk_string("myset"),
            RespValue::bulk_string("b"),
            RespValue::bulk_string("c"),
            RespValue::bulk_string("d"),
        ];
        let result = cmd.execute(&mut ctx, &args);
        assert_eq!(result, RespValue::integer(1)); // Only d was added
    }

    #[test]
    fn test_smembers() {
        let mut ctx = CommandContext::new();
        let sadd_cmd = SAddCommand;
        let smembers_cmd = SMembersCommand;

        // SADD myset a b c
        let args = vec![
            RespValue::bulk_string("myset"),
            RespValue::bulk_string("a"),
            RespValue::bulk_string("b"),
            RespValue::bulk_string("c"),
        ];
        sadd_cmd.execute(&mut ctx, &args);

        // SMEMBERS myset
        let args = vec![RespValue::bulk_string("myset")];
        let result = smembers_cmd.execute(&mut ctx, &args);

        // Check that we got an array with 3 elements
        if let RespValue::Array(arr) = result {
            assert_eq!(arr.len(), 3);
            // Can't guarantee order in a set
        } else {
            panic!("Expected array response");
        }

        // SMEMBERS nonexistent
        let args = vec![RespValue::bulk_string("nonexistent")];
        let result = smembers_cmd.execute(&mut ctx, &args);
        assert_eq!(result, RespValue::array(vec![]));
    }

    #[test]
    fn test_scard() {
        let mut ctx = CommandContext::new();
        let sadd_cmd = SAddCommand;
        let scard_cmd = SCardCommand;

        // SADD myset a b c
        let args = vec![
            RespValue::bulk_string("myset"),
            RespValue::bulk_string("a"),
            RespValue::bulk_string("b"),
            RespValue::bulk_string("c"),
        ];
        sadd_cmd.execute(&mut ctx, &args);

        // SCARD myset
        let args = vec![RespValue::bulk_string("myset")];
        let result = scard_cmd.execute(&mut ctx, &args);
        assert_eq!(result, RespValue::integer(3));

        // SCARD nonexistent
        let args = vec![RespValue::bulk_string("nonexistent")];
        let result = scard_cmd.execute(&mut ctx, &args);
        assert_eq!(result, RespValue::integer(0));
    }
}
