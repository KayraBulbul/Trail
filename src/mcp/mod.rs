//! `trail mcp`: an MCP server over stdio. See the MCP plan in CLAUDE.md.

use std::{
    io::{self, BufRead, Write},
    path::Path,
};

use rusqlite::Connection;
use serde_json::{Value, json};

pub mod tools;

/// Reads JSON-RPC messages from `input`, one per line, and writes replies to
/// `output`, one per line, until `input` ends. `trail mcp` passes stdin and
/// stdout; tests pass a string and a buffer.
pub fn serve(
    conn: &Connection,
    cwd: &Path,
    input: impl BufRead,
    mut output: impl Write,
) -> io::Result<()> {
    let _ = (conn, cwd);

    for line in input.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        let message: Value = match serde_json::from_str(&line) {
            Ok(message) => message,
            Err(_) => {
                send(&mut output, &error(Value::Null, -32700, "Parse error"))?;
                continue;
            }
        };

        let Some(id) = message.get("id").cloned() else {
            continue;
        };

        let Some(method) = message["method"].as_str() else {
            send(&mut output, &error(id, -32600, "Invalid request"))?;
            continue;
        };

        match method {
            _ => send(&mut output, &error(id, -32601, "Method not found"))?,
        };
    }

    Ok(())
}

fn send(output: &mut impl Write, reply: &Value) -> io::Result<()> {
    let reply = serde_json::to_string(reply)?;
    writeln!(output, "{}", reply)?;
    output.flush()
}

fn result(id: Value, result: Value) -> Value {
    json!(
        {
            "jsonrpc": "2.0",
            "id": id,
            "result": result
        }
    )
}

fn error(id: Value, code: i64, message: &str) -> Value {
    json!(
        {
            "jsonrpc": "2.0",
            "id": id,
            "error": {
                "code": code,
                "message": message
            }
        }
    )
}

#[cfg(test)]
mod tests;
