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
        r#"{"jsonrpc":"2.0","id":1,"method":"nope"}"#,
        r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
        "",
        r#"{"jsonrpc":"2.0","id":"two","method":"nope"}"#,
    ]);

    let ids: Vec<&Value> = replies.iter().map(|reply| &reply["id"]).collect();
    assert_eq!(ids, [&json!(1), &json!("two")]);
    for reply in &replies {
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
