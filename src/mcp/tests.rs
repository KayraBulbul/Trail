use super::*;
use crate::database::sqlite;

/// Feeds `lines` to `serve` and returns each reply line parsed as JSON.
fn session(lines: &[&str]) -> Vec<Value> {
    let conn = Connection::open_in_memory().unwrap();
    sqlite::initialize_schema(&conn).unwrap();
    let input = lines.join("\n");
    let mut output = Vec::new();

    serve(&conn, Path::new("/"), input.as_bytes(), &mut output).unwrap();

    String::from_utf8(output)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

#[test]
fn every_request_gets_one_reply_with_its_id_and_notifications_get_none() {
    let replies = session(&[
        r#"{"jsonrpc":"2.0","id":0,"method":"initialize","params":{"protocolVersion":"2025-06-18"}}"#,
        r#"{"jsonrpc":"2.0","id":1,"method":"nope"}"#,
        r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
        "",
        r#"{"jsonrpc":"2.0","id":"two","method":"nope"}"#,
    ]);

    let ids: Vec<&Value> = replies.iter().map(|reply| &reply["id"]).collect();
    assert_eq!(ids, [&json!(0), &json!(1), &json!("two")]);
    for reply in &replies[1..] {
        assert_eq!(reply["jsonrpc"], "2.0");
        assert_eq!(reply["error"]["code"], -32601);
    }
}

#[test]
fn malformed_messages_get_json_rpc_errors() {
    let replies = session(&[
        "not json",
        r#"{"jsonrpc":"2.0","id":3}"#,
        r#"{"jsonrpc":"2.0","id":4,"method":5}"#,
    ]);

    let errors: Vec<(&Value, &Value)> = replies
        .iter()
        .map(|reply| (&reply["id"], &reply["error"]["code"]))
        .collect();
    assert_eq!(
        errors,
        [
            (&Value::Null, &json!(-32700)),
            (&json!(3), &json!(-32600)),
            (&json!(4), &json!(-32600)),
        ]
    );
}

#[test]
fn ends_cleanly_when_input_closes() {
    assert!(session(&[]).is_empty());
}

const MODERN_META: &str = r#""_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientCapabilities":{}}"#;

#[test]
fn legacy_clients_must_initialize_first_and_get_their_version_echoed() {
    let replies = session(&[
        r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#,
        r#"{"jsonrpc":"2.0","id":2,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{}}}"#,
        r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
        r#"{"jsonrpc":"2.0","id":3,"method":"ping"}"#,
    ]);

    assert_eq!(replies.len(), 3);
    assert_eq!(replies[0]["error"]["code"], -32602);
    let initialized = &replies[1]["result"];
    assert_eq!(initialized["protocolVersion"], "2025-06-18");
    assert_eq!(initialized["capabilities"], json!({ "tools": {} }));
    assert_eq!(initialized["serverInfo"]["name"], "trail");
    assert_eq!(replies[2]["result"]["resultType"], "complete");
}

#[test]
fn initialize_with_an_unknown_version_answers_with_the_newest_legacy_one() {
    let replies = session(&[
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-01-01","capabilities":{}}}"#,
    ]);

    assert_eq!(replies[0]["result"]["protocolVersion"], LEGACY_VERSIONS[0]);
}

#[test]
fn modern_requests_work_without_initialize() {
    let replies = session(&[
        &format!(
            r#"{{"jsonrpc":"2.0","id":1,"method":"server/discover","params":{{{MODERN_META}}}}}"#
        ),
        &format!(r#"{{"jsonrpc":"2.0","id":2,"method":"ping","params":{{{MODERN_META}}}}}"#),
    ]);

    let discover = &replies[0]["result"];
    assert_eq!(discover["resultType"], "complete");
    assert_eq!(discover["supportedVersions"][0], "2026-07-28");
    assert_eq!(discover["capabilities"], json!({ "tools": {} }));
    assert_eq!(
        discover["_meta"]["io.modelcontextprotocol/serverInfo"]["name"],
        "trail"
    );
    assert!(replies[1]["error"].is_null());
}

#[test]
fn modern_requests_need_a_supported_version_and_capabilities() {
    let replies = session(&[
        r#"{"jsonrpc":"2.0","id":1,"method":"ping","params":{"_meta":{"io.modelcontextprotocol/protocolVersion":"1999-01-01","io.modelcontextprotocol/clientCapabilities":{}}}}"#,
        r#"{"jsonrpc":"2.0","id":2,"method":"ping","params":{"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28"}}}"#,
    ]);

    let unsupported = &replies[0]["error"];
    assert_eq!(unsupported["code"], -32022);
    assert_eq!(unsupported["data"]["requested"], "1999-01-01");
    assert_eq!(
        unsupported["data"]["supported"].as_array().unwrap().len(),
        4
    );
    assert_eq!(replies[1]["error"]["code"], -32602);
}

#[test]
fn tools_list_and_call_work_in_both_eras() {
    let replies = session(&[
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18"}}"#,
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#,
        r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"trail_projects"}}"#,
        &format!(
            r#"{{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{{"name":"trail_projects","arguments":{{}},{MODERN_META}}}}}"#
        ),
    ]);

    assert_eq!(replies[1]["result"]["tools"], tools::definitions());
    for reply in &replies[2..] {
        assert_eq!(reply["result"]["isError"], false);
        assert_eq!(reply["result"]["content"][0]["text"], "[]");
        assert_eq!(reply["result"]["resultType"], "complete");
    }
}

#[test]
fn tool_failures_are_results_but_bad_calls_are_errors() {
    let replies = session(&[
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18"}}"#,
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"trail_status"}}"#,
        r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"arguments":{}}}"#,
        r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"trail_delete"}}"#,
    ]);

    assert_eq!(replies[1]["result"]["isError"], true);
    assert_eq!(replies[2]["error"]["code"], -32602);
    assert_eq!(replies[3]["error"]["code"], -32602);
    assert!(
        replies[3]["error"]["message"]
            .as_str()
            .unwrap()
            .contains("trail_delete")
    );
}
