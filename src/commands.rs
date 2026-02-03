use crate::resp::RespValue;

#[derive(Debug)]
pub enum Command {
    Ping,
    ECHO(String),
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
            _ => Err(format!("Unknown command: {}", command_name)),
        }
    }

    pub fn execute(&self) -> RespValue {
        match self {
            Command::Ping => RespValue::SimpleString("PONG".to_string()),
            Command::ECHO(msg) => RespValue::BulkString(msg.clone()),
        }
    }
}

fn parse_echo(args: &[RespValue]) -> Result<Command, String> {
    if args.len() != 2 {
        return Err("ERR wrong number of arguments for 'echo' command".to_string());
    }

    match &args[1] {
        RespValue::BulkString(s) => Ok(Command::ECHO(s.clone())),
        _ => Err("ERR argument must be a string".to_string()),
    }
}
