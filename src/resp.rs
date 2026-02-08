use std::io::{Cursor, Read};

#[derive(Debug, Clone, PartialEq)]
pub enum RespValue {
    SimpleString(String),  // +OK\r\n
    SimpleError(String),   // -Error message\r\n
    Integer(i64),          // :[<+|->]<value>\r\n
    BulkString(String),    // $<length>\r\n<data>\r\n
    Array(Vec<RespValue>), // *<number-of-elements>\r\n<element-1>...<element-n>
    Null,
}

const CRLF: &[u8] = b"\r\n";

impl RespValue {
    pub fn serialize(&self) -> Vec<u8> {
        match self {
            RespValue::SimpleString(s) => format!("+{}\r\n", s).into_bytes(),
            RespValue::SimpleError(s) => format!("-{}\r\n", s).into_bytes(),
            RespValue::Integer(i) => format!(":{}\r\n", i).into_bytes(),
            RespValue::BulkString(s) => format!("${}\r\n{}\r\n", s.len(), s).into_bytes(),
            RespValue::Null => b"$-1\r\n".to_vec(),
            RespValue::Array(arr) => {
                let mut buf = Vec::new();
                buf.extend_from_slice(format!("*{}\r\n", arr.len()).as_bytes());
                for item in arr {
                    buf.extend_from_slice(item.serialize().as_ref());
                }
                buf
            }
        }
    }
}

pub fn parse_resp(cursor: &mut Cursor<&[u8]>) -> Result<RespValue, String> {
    let mut type_byte = [0; 1];

    if cursor
        .read(&mut type_byte)
        .map_err(|_| "Failed to read type byte")?
        == 0
    {
        return Err("EOF".to_string());
    }

    match type_byte[0] {
        b'+' => parse_simple_string(cursor),
        b'-' => parse_error(cursor),
        b':' => parse_integer(cursor),
        b'$' => parse_bulk_string(cursor),
        b'*' => parse_array(cursor),
        _ => Err(format!("Unknown RESP type: {}", type_byte[0] as char)),
    }
}

fn read_line(cursor: &mut Cursor<&[u8]>) -> Result<String, String> {
    let position = cursor.position() as usize;
    let inner = *cursor.get_ref();

    if position >= inner.len() {
        return Err("Incomplete".to_string());
    }

    for i in position..inner.len() - 1 {
        if &inner[i..i + 2] == CRLF {
            cursor.set_position((i + 2) as u64); //consume CLRF

            return Ok(String::from_utf8_lossy(&inner[position..i]).to_string());
        }
    }

    Err("Incomplete".to_string())
}

fn parse_simple_string(cursor: &mut Cursor<&[u8]>) -> Result<RespValue, String> {
    let s = read_line(cursor)?;
    Ok(RespValue::SimpleString(s))
}

fn parse_error(cursor: &mut Cursor<&[u8]>) -> Result<RespValue, String> {
    let s = read_line(cursor)?;
    Ok(RespValue::SimpleError(s))
}

fn parse_integer(cursor: &mut Cursor<&[u8]>) -> Result<RespValue, String> {
    let s = read_line(cursor)?;
    let i = s.parse::<i64>().map_err(|_| "Invalid integer")?;
    Ok(RespValue::Integer(i))
}

fn parse_bulk_string(cursor: &mut Cursor<&[u8]>) -> Result<RespValue, String> {
    let len_str = read_line(cursor)?;
    let len = len_str
        .parse::<i64>()
        .map_err(|_| "Invalid bulk string length")?;

    // Handle Null Bulk String ($-1\r\n)
    if len == -1 {
        return Ok(RespValue::Null);
    }

    let len = len as usize;
    let mut buf = vec![0; len];

    cursor
        .read_exact(&mut buf)
        .map_err(|_| "Failed to read bulk string data")?;

    let mut crlf = [0; 2];
    cursor
        .read_exact(&mut crlf)
        .map_err(|_| "Failed to read CRLF")?;
    if crlf != CRLF {
        return Err("Invalid bulk string ending".to_string());
    }

    let s = String::from_utf8_lossy(&buf).to_string();

    Ok(RespValue::BulkString(s))
}

fn parse_array(cursor: &mut Cursor<&[u8]>) -> Result<RespValue, String> {
    let size = read_line(cursor)?;
    let array_len = size.parse::<i64>().map_err(|_| "Invalid array length")?;

    if array_len == -1 {
        return Ok(RespValue::Null);
    }

    let mut items = Vec::with_capacity(array_len as usize);
    for _ in 0..array_len {
        let item = parse_resp(cursor)?;
        items.push(item);
    }

    Ok(RespValue::Array(items))
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_parse_array_echo_hey() {
        // "*2\r\n$4\r\nECHO\r\n$3\r\nhey\r\n"
        // "*2" = Array of length 2
        // "$4\r\nECHO\r\n" = Bulk String "ECHO"
        // "$3\r\nhey\r\n"  = Bulk String "hey"
        let input = b"*2\r\n$4\r\nECHO\r\n$3\r\nhey\r\n";
        let mut cursor = Cursor::new(&input[..]);

        let result = parse_resp(&mut cursor).unwrap();
        let expected = RespValue::Array(vec![
            RespValue::BulkString("ECHO".to_string()),
            RespValue::BulkString("hey".to_string()),
        ]);

        assert_eq!(result, expected);
    }
    #[test]
    fn test_parse_simple_string() {
        let input = b"+OK\r\n";
        let mut cursor = Cursor::new(&input[..]);
        let result = parse_resp(&mut cursor).unwrap();
        assert_eq!(result, RespValue::SimpleString("OK".to_string()));
    }

    #[test]
    fn test_parse_integer() {
        let input = b":1000\r\n";
        let mut cursor = Cursor::new(&input[..]);
        let result = parse_resp(&mut cursor).unwrap();
        assert_eq!(result, RespValue::Integer(1000));
    }

    #[test]
    fn test_parse_bulk_string() {
        let input = b"$5\r\nhello\r\n";
        let mut cursor = Cursor::new(&input[..]);
        let result = parse_resp(&mut cursor).unwrap();
        assert_eq!(result, RespValue::BulkString("hello".to_string()));
    }

    #[test]
    fn test_parse_null_bulk_string() {
        let input = b"$-1\r\n";
        let mut cursor = Cursor::new(&input[..]);
        let result = parse_resp(&mut cursor).unwrap();
        assert_eq!(result, RespValue::Null);
    }

    #[test]
    fn test_parse_array_mixed() {
        // Array of [Integer(1), SimpleString("OK")]
        let input = b"*2\r\n:1\r\n+OK\r\n";
        let mut cursor = Cursor::new(&input[..]);
        let result = parse_resp(&mut cursor).unwrap();

        let expected = RespValue::Array(vec![
            RespValue::Integer(1),
            RespValue::SimpleString("OK".to_string()),
        ]);
        assert_eq!(result, expected);
    }
}
