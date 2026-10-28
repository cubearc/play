use crate::Result;
use serde::{Deserialize, Serialize};
use std::io::{BufRead, Read, Write};

#[derive(Deserialize)]
pub struct Contract {
    pub protocol: u32,
    pub engine: String,
    pub frame_bytes: usize,
    pub input_bytes: usize,
    pub queue_commands: usize,
    pub handshake_ms: u64,
    pub run_ms: u64,
    pub cpu_seconds: u64,
    pub vm_growth_bytes: u64,
    pub output_bytes: usize,
}

pub fn contract() -> Contract {
    serde_json::from_str(include_str!("../../../desktop/protocol.json"))
        .expect("reviewed desktop protocol contract")
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Command {
    Hello {
        id: u32,
        protocol: u32,
        engine: String,
        nonce: String,
    },
    Invoke {
        id: u32,
        invocation: Invocation,
    },
    Catalog {
        id: u32,
    },
    Cancel {
        id: u32,
        run_id: u32,
    },
}

impl Command {
    pub fn id(&self) -> u32 {
        match self {
            Self::Hello { id, .. }
            | Self::Invoke { id, .. }
            | Self::Catalog { id }
            | Self::Cancel { id, .. } => *id,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Invocation {
    pub operation: String,
    pub operation_version: u32,
    pub input: String,
}

pub fn read_frame(reader: &mut impl BufRead) -> Result<Option<Vec<u8>>> {
    let max = contract().frame_bytes;
    let mut frame = Vec::new();
    let length = reader
        .take(max as u64 + 1)
        .read_until(b'\n', &mut frame)
        .map_err(|_| "transport_closed")?;
    if length == 0 {
        return Ok(None);
    }
    if length > max || frame.last() != Some(&b'\n') {
        return Err("invalid_frame");
    }
    Ok(Some(frame))
}

pub fn parse(frame: &[u8]) -> Result<Command> {
    let text = std::str::from_utf8(frame).map_err(|_| "invalid_frame")?;
    let value = cube_core::pipeline::json::parse(text).map_err(|_| "invalid_frame")?;
    serde_json::from_value(value).map_err(|_| "invalid_command")
}

pub fn write_frame(writer: &mut impl Write, value: &impl Serialize) -> Result<()> {
    let mut bytes = serde_json::to_vec(value).map_err(|_| "invalid_response")?;
    bytes.push(b'\n');
    if bytes.len() > contract().frame_bytes {
        return Err("output_limit");
    }
    writer
        .write_all(&bytes)
        .and_then(|()| writer.flush())
        .map_err(|_| "transport_closed")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_ambiguous_or_unbounded_commands() {
        for document in [
            r#"{"type":"cancel","id":1,"id":2,"run_id":3}"#,
            r#"{"type":"cancel","id":1,"run_id":3,"path":"secret"}"#,
            r#"{"type":"shell","id":1,"command":"pwd"}"#,
        ] {
            assert!(parse(document.as_bytes()).is_err());
        }
        assert!(read_frame(&mut &b"no terminator"[..]).is_err());
        assert!(read_frame(&mut vec![b'x'; contract().frame_bytes + 1].as_slice()).is_err());
        assert!(read_frame(&mut &b""[..]).unwrap().is_none());
    }

    #[test]
    fn pins_engine_to_the_built_binary() {
        assert_eq!(contract().engine, env!("CARGO_PKG_VERSION"));
        assert!(contract().input_bytes * 6 < contract().frame_bytes);
        assert!(write_frame(&mut Vec::new(), &"x".repeat(contract().frame_bytes)).is_err());
    }
}
