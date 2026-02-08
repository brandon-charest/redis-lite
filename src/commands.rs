use std::{
    collections::btree_map::Keys,
    time::{Duration, Instant},
};

use crate::{
    db::{DataType, Db},
    resp::RespValue,
};

#[derive(Debug)]
pub enum Command {
    Ping,
    Echo(String),
    Set(String, String, Option<Duration>),
    Get(String),
    RPush(String, String),
}

impl Command {
    pub fn from_resp(value: RespValue) -> Result<Command, String> {
        let args = match value {
            RespValue::Array(a) => a,
            _ => return Err("Command must be an Array".to_string()),
        };

        if args.is_empty() {
            return Err("Command cannot be empty".to_string());
        }

        let command_name = match &args[0] {
            RespValue::SimpleString(s) | RespValue::BulkString(s) => s.to_uppercase(),
            _ => return Err("Command name must be a string".to_string()),
        };

        match command_name.as_str() {
            "PING" => Ok(Command::Ping),
            "ECHO" => parse_echo(&args),
            "SET" => parse_set(&args),
            "GET" => parse_get(&args),
            "RPUSH" => parse_rpush(&args),
            _ => Err(format!("Unknown command: {}", command_name)),
        }
    }

    pub fn execute(&self, db: &Db) -> RespValue {
        match self {
            Command::Ping => RespValue::SimpleString("PONG".to_string()),
            Command::Echo(msg) => RespValue::BulkString(msg.clone()),
            Command::Set(key, value, duration) => {
                let expiry = duration.map(|d| Instant::now() + d);
                db.set(key.clone(), value.clone(), expiry);
                RespValue::SimpleString("OK".to_string())
            }
            Command::Get(key) => match db.get(key) {
                Some(DataType::String(s)) => RespValue::BulkString(s),
                None => RespValue::Null,
                _ => RespValue::SimpleError(
                    "WRONGTYPE Operation against a key holding the wrong kind of value".to_string(),
                ),
            },
            Command::RPush(key, value) => {
                let curr_len = db.rpush(key.clone(), value.clone());
                if curr_len == 0 {
                    RespValue::SimpleError(
                        "WRONGTYPE Operation against a key holding the wrong kind of value"
                            .to_string(),
                    )
                } else {
                    RespValue::Integer(curr_len as i64)
                }
            }
        }
    }
}

fn parse_echo(args: &[RespValue]) -> Result<Command, String> {
    match args.get(1) {
        Some(RespValue::BulkString(s)) => Ok(Command::Echo(s.clone())),
        _ => Err("ERR wrong number of arguments for 'echo' command".to_string()),
    }
}

fn parse_set(args: &[RespValue]) -> Result<Command, String> {
    if args.len() < 3 {
        return Err("ERR wrong number of arguments for 'set' command".to_string());
    }

    let key = get_bulk_string_value(&args[1]);
    let value = get_bulk_string_value(&args[2]);

    let mut duration: Option<Duration> = None;

    if args.len() > 3 {
        match &args[3] {
            RespValue::BulkString(s) if s.to_lowercase() == "px" => match args.get(4) {
                Some(RespValue::BulkString(ms_str)) => {
                    let ms = ms_str
                        .parse::<u64>()
                        .map_err(|_| "ERR value is not an integer")?;
                    duration = Some(Duration::from_millis(ms));
                }
                _ => return Err("ERR syntax error".to_string()),
            },
            _ => return Err("ERR syntax error".to_string()),
        }
    }

    Ok(Command::Set(key?, value?, duration))
}

fn parse_get(args: &[RespValue]) -> Result<Command, String> {
    if args.len() != 2 {
        return Err("ERR wrong number of arguments for 'get' command".to_string());
    }

    let key = get_bulk_string_value(&args[1]);

    Ok(Command::Get(key?))
}

fn parse_rpush(args: &[RespValue]) -> Result<Command, String> {
    if args.len() < 3 {
        return Err("ERR wrong number of arguments for 'set' command".to_string());
    }

    let key = get_bulk_string_value(&args[1]);
    let value = get_bulk_string_value(&args[2]);
    Ok(Command::RPush(key?, value?))
}

fn get_bulk_string_value(arg: &RespValue) -> Result<String, String> {
    Ok(match arg {
        RespValue::BulkString(s) => s.clone(),
        _ => return Err("ERR value must be bulk string".to_string()),
    })
}
