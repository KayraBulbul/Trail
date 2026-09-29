//! `trail mcp`: an MCP server over stdio. See the MCP plan in CLAUDE.md.

use std::{
    io::{self, BufRead, Write},
    path::Path,
};

use rusqlite::Connection;
use serde_json::{Value, json};

pub mod tools;

const MODERN_VERSIONS: &[&str] = &["2026-07-28"];
const LEGACY_VERSIONS: &[&str] = &["2025-11-25", "2025-06-18", "2025-03-26"];

/// Reads JSON-RPC messages from `input`, one per line, and writes replies to
/// `output`, one per line, until `input` ends. `trail mcp` passes stdin and
/// stdout; tests pass a string and a buffer.
pub fn serve(
    conn: &Connection,
    cwd: &Path,
    input: impl BufRead,
    mut output: impl Write,
) -> io::Result<()> {
    let mut initialized = false;

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

        if method == "initialize" {
            let protocol_version = message["params"]["protocolVersion"]
                .as_str()
                .filter(|version| LEGACY_VERSIONS.contains(version))
                .unwrap_or(LEGACY_VERSIONS[0]);
            let response = result(
                id,
                json!(
                    {
                        "protocolVersion": protocol_version,
                        "capabilities": {
                            "tools": {}
                        },
                        "serverInfo": {
                            "name": "trail",
                            "version": env!("CARGO_PKG_VERSION")
                        }
                    }
                ),
            );
            send(&mut output, &response)?;
            initialized = true;
            continue;
        }

        if let Err(reply) = check_version(&message, &id, initialized) {
            send(&mut output, &reply)?;
            continue;
        }

        match method {
            "server/discover" => {
                let versions = [MODERN_VERSIONS, LEGACY_VERSIONS].concat();
                let reply = json!(
                            {
                                "supportedVersions": versions,
                                "capabilities": {
                                    "tools": {}
                                },
                                "_meta": {
                                "io.modelcontextprotocol/serverInfo": {
                                    "name": "trail", "version": env!("CARGO_PKG_VERSION")
                                }
                            }
                            }
                );
                send(&mut output, &result(id, reply))?;
            }
            "ping" => send(&mut output, &result(id, json!({})))?,
            "tools/list" => send(
                &mut output,
                &result(id, json!({"tools": &tools::definitions()})),
            )?,
            "tools/call" => {
                let Some(tool_name) = message["params"]["name"].as_str() else {
                    send(&mut output, &error(id, -32602, "Missing tool name"))?;
                    continue;
                };
                match tools::call(conn, cwd, tool_name, &message["params"]["arguments"]) {
                    Some(tool_result) => send(&mut output, &result(id, tool_result))?,
                    None => send(
                        &mut output,
                        &error(id, -32602, &format!("Unknown tool: {tool_name}")),
                    )?,
                }
            }
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

fn check_version(message: &Value, id: &Value, initialized: bool) -> Result<(), Value> {
    let meta = &message["params"]["_meta"];
    let versions = [MODERN_VERSIONS, LEGACY_VERSIONS].concat();
    match meta["io.modelcontextprotocol/protocolVersion"].as_str() {
        Some(version) if !MODERN_VERSIONS.contains(&version) => Err(error_with_data(
            id.clone(),
            -32022,
            "Unsupported protocol version",
            json!({ "supported": versions, "requested": version }),
        )),
        Some(_) if meta["io.modelcontextprotocol/clientCapabilities"].is_null() => Err(error(
            id.clone(),
            -32602,
            "Missing io.modelcontextprotocol/clientCapabilities in _meta",
        )),
        Some(_) => Ok(()),
        None if initialized => Ok(()),
        None => Err(error(
            id.clone(),
            -32602,
            "Missing protocol version: send initialize first, or include io.modelcontextprotocol/protocolVersion in _meta",
        )),
    }
}

fn result(id: Value, mut result: Value) -> Value {
    result["resultType"] = json!("complete");
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

fn error_with_data(id: Value, code: i64, message: &str, data: Value) -> Value {
    json!(
        {
            "jsonrpc": "2.0",
            "id": id,
            "error": {
                "code": code,
                "message": message,
                "data": data
            }
        }
    )
}

#[cfg(test)]
mod tests;
