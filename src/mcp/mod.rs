//! `trail mcp`: an MCP server over stdio. See the MCP plan in CLAUDE.md.

use std::{
    io::{self, BufRead, Write},
    path::Path,
};

use rusqlite::Connection;

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
    // TODO: the protocol layer (steps 2–4 in CLAUDE.md).
    let _ = (conn, cwd, input);
    output.flush()
}
