//! Admin commands (INFO, FLUSHDB)

use super::{Command, CommandContext};
use crate::protocol::RespValue;

/// INFO command - Get information and statistics about the server
///
/// Syntax: INFO [section]
pub struct InfoCommand;

impl Command for InfoCommand {
    fn execute(&self, ctx: &mut CommandContext, _args: &[RespValue]) -> RespValue {
        let stats = ctx.store.stats();

        let info = format!(
            "# Server\r\n\
            ferrumdb_version:0.1.0\r\n\
            ferrumdb_mode:standalone\r\n\
            os:{}\r\n\
            arch:{}\r\n\
            \r\n\
            # Keyspace\r\n\
            db0:keys={},expires={}\r\n",
            std::env::consts::OS,
            std::env::consts::ARCH,
            stats.active_keys,
            stats.expired_keys
        );

        RespValue::bulk_string(info)
    }

    fn name(&self) -> &'static str {
        "INFO"
    }

    fn min_args(&self) -> usize {
        0
    }

    fn max_args(&self) -> Option<usize> {
        Some(1)
    }
}

/// FLUSHDB command - Remove all keys from the current database
///
/// Syntax: FLUSHDB
pub struct FlushDbCommand;

impl Command for FlushDbCommand {
    fn execute(&self, ctx: &mut CommandContext, _args: &[RespValue]) -> RespValue {
        ctx.store.clear();
        RespValue::simple_string("OK")
    }

    fn name(&self) -> &'static str {
        "FLUSHDB"
    }

    fn min_args(&self) -> usize {
        0
    }

    fn max_args(&self) -> Option<usize> {
        Some(0)
    }
}

/// CLIENT command - Client connection commands
///
/// Syntax: CLIENT <subcommand> [args...]
/// Subcommands:
/// - SETNAME <name>: Set client name
/// - GETNAME: Get client name
/// - LIST: List client connections
/// - SETINFO: Set client info (stub)
pub struct ClientCommand;

impl Command for ClientCommand {
    fn execute(&self, _ctx: &mut CommandContext, args: &[RespValue]) -> RespValue {
        if args.is_empty() {
            return RespValue::error("ERR wrong number of arguments for 'client' command");
        }

        // Extract subcommand
        let subcommand = match &args[0] {
            RespValue::BulkString(bytes) => {
                match std::str::from_utf8(bytes) {
                    Ok(s) => s.to_uppercase(),
                    Err(_) => return RespValue::error("ERR invalid subcommand"),
                }
            }
            _ => return RespValue::error("ERR invalid subcommand"),
        };

        match subcommand.as_str() {
            "SETNAME" => {
                // CLIENT SETNAME <name>
                // In a sharded architecture, we don't track per-client state
                // Just return OK to maintain compatibility
                if args.len() != 2 {
                    return RespValue::error("ERR wrong number of arguments for 'client setname'");
                }
                RespValue::simple_string("OK")
            }
            "GETNAME" => {
                // CLIENT GETNAME
                // Return null since we don't track client names
                RespValue::Null
            }
            "LIST" => {
                // CLIENT LIST
                // Return empty list since we don't track connections in shards
                RespValue::bulk_string("")
            }
            "SETINFO" => {
                // CLIENT SETINFO <attr> <value>
                // Used by some clients for library info
                // Just return OK
                RespValue::simple_string("OK")
            }
            "REPLY" => {
                // CLIENT REPLY ON|OFF|SKIP
                // Return OK (we always send replies)
                RespValue::simple_string("OK")
            }
            "ID" => {
                // CLIENT ID
                // Return a dummy ID
                RespValue::Integer(1)
            }
            _ => {
                RespValue::error(format!("ERR unknown subcommand '{}'", subcommand))
            }
        }
    }

    fn name(&self) -> &'static str {
        "CLIENT"
    }

    fn min_args(&self) -> usize {
        1
    }

    fn max_args(&self) -> Option<usize> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::Value;

    #[test]
    fn test_info() {
        let mut ctx = CommandContext::new();
        ctx.store.set("key1", Value::string("value1"));
        ctx.store.set("key2", Value::string("value2"));

        let cmd = InfoCommand;
        let result = cmd.execute(&mut ctx, &[]);

        if let RespValue::BulkString(bytes) = result {
            let info = String::from_utf8(bytes.to_vec()).unwrap();
            assert!(info.contains("ferrumdb_version"));
            assert!(info.contains("keys=2"));
        } else {
            panic!("Expected bulk string response");
        }
    }

    #[test]
    fn test_flushdb() {
        let mut ctx = CommandContext::new();
        ctx.store.set("key1", Value::string("value1"));
        ctx.store.set("key2", Value::string("value2"));

        assert_eq!(ctx.store.len(), 2);

        let cmd = FlushDbCommand;
        let result = cmd.execute(&mut ctx, &[]);
        assert_eq!(result, RespValue::simple_string("OK"));

        assert_eq!(ctx.store.len(), 0);
    }
}
