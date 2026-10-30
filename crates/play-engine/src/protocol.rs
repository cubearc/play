use crate::Result;
use serde::{Deserialize, Serialize};
use std::io::{BufRead, Read, Write};

#[derive(Deserialize)]
pub struct Contract {
    pub parallel_nodes: usize,
    pub pipeline_node_limit: usize,
    pub pipeline_input_limit: usize,
    pub pipeline_definition_bytes: usize,
    pub pipeline_literal_bytes: usize,
    pub pipeline_canvas_limit: u16,
    pub pipeline_limit: usize,
    pub pipeline_revision_limit: usize,
    pub pipeline_parameter_bytes: usize,
    pub hypothesis_bytes: usize,
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
    pub storage_generation: u32,
    pub schema_version: u32,
    pub workspace_limit: usize,
    pub exploration_limit: usize,
    pub stage_limit: usize,
    pub block_limit: usize,
    pub retained_block_limit: usize,
    pub text_bytes: usize,
    pub note_bytes: usize,
    pub image_bytes: usize,
    pub file_bytes: usize,
    pub file_preview_bytes: usize,
    pub store_bytes: usize,
    pub exploration_bytes: usize,
    pub workspace_run_limit: usize,
    pub exploration_run_limit: usize,
    pub run_page_limit: usize,
    pub run_reserve_bytes: usize,
}

pub fn contract() -> Contract {
    serde_json::from_str(include_str!("../../../desktop/protocol.json"))
        .expect("reviewed desktop protocol contract")
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Command {
    Terminal {
        id: u32,
        command: crate::terminal::Command,
    },
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
    Workspace {
        id: u32,
        command: crate::workspace::Command,
    },
    StartRun {
        id: u32,
        request: crate::workspace::RunRequest,
    },
    StartCliRun {
        id: u32,
        request: crate::workspace::RunRequest,
    },
    CliRead {
        id: u32,
        context: crate::workspace::Context,
        command: crate::workspace::Read,
    },
    CancelRun {
        id: u32,
        context: crate::workspace::Context,
        run_id: uuid::Uuid,
    },
    Cancel {
        id: u32,
        run_id: u32,
    },
}

impl Command {
    pub fn id(&self) -> u32 {
        match self {
            Self::Terminal { id, .. } => *id,
            Self::Hello { id, .. }
            | Self::Invoke { id, .. }
            | Self::Catalog { id }
            | Self::Cancel { id, .. }
            | Self::Workspace { id, .. }
            | Self::StartRun { id, .. }
            | Self::StartCliRun { id, .. }
            | Self::CliRead { id, .. }
            | Self::CancelRun { id, .. } => *id,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Invocation {
    pub parameters: serde_json::Value,
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
