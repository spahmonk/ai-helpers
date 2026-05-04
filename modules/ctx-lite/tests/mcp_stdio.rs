use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, ChildStdout, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

use serde_json::{json, Value};

fn spawn_mcp_server() -> (Child, Receiver<String>) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_ctx-lite"))
        .arg("--mcp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn ctx-lite --mcp");

    let stdout = child.stdout.take().expect("missing stdout");
    let responses = spawn_response_reader(stdout);

    (child, responses)
}

fn spawn_raw_mcp_server() -> Child {
    Command::new(env!("CARGO_BIN_EXE_ctx-lite"))
        .arg("--mcp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn ctx-lite --mcp")
}

fn spawn_response_reader(stdout: ChildStdout) -> Receiver<String> {
    let (sender, receiver) = mpsc::channel();

    std::thread::spawn(move || {
        let mut reader = BufReader::new(stdout);

        loop {
            let Some(body) = read_response_body(&mut reader) else {
                return;
            };

            if sender
                .send(String::from_utf8(body).expect("invalid utf-8"))
                .is_err()
            {
                return;
            }
        }
    });

    receiver
}

fn read_response_body(reader: &mut BufReader<ChildStdout>) -> Option<Vec<u8>> {
    let mut first_line = String::new();
    let bytes = reader.read_line(&mut first_line).ok()?;
    if bytes == 0 {
        return None;
    }

    let trimmed = first_line.trim_end_matches(['\r', '\n']);
    if trimmed.starts_with('{') {
        return Some(trimmed.as_bytes().to_vec());
    }

    let mut content_length = capture_content_length(&first_line);

    loop {
        let mut line = String::new();
        let bytes = reader.read_line(&mut line).ok()?;
        if bytes == 0 {
            return None;
        }

        if line.trim_end_matches(['\r', '\n']).is_empty() {
            break;
        }

        content_length = content_length.or_else(|| capture_content_length(&line));
    }

    let length = content_length?;
    let mut body = vec![0_u8; length];
    reader.read_exact(&mut body).ok()?;
    Some(body)
}

fn capture_content_length(line: &str) -> Option<usize> {
    let (name, value) = line.split_once(':')?;
    if name.trim().eq_ignore_ascii_case("Content-Length") {
        return value.trim().parse::<usize>().ok();
    }

    None
}

fn send_request(child: &mut Child, request: Value) {
    let body = serde_json::to_vec(&request).expect("request should serialize");
    let stdin = child.stdin.as_mut().expect("missing stdin");
    write!(stdin, "Content-Length: {}\r\n\r\n", body.len()).expect("failed to write header");
    stdin.write_all(&body).expect("failed to write body");
    stdin.flush().expect("failed to flush request");
}

fn send_request_with_lf_only(child: &mut Child, request: Value) {
    let body = serde_json::to_vec(&request).expect("request should serialize");
    let stdin = child.stdin.as_mut().expect("missing stdin");
    write!(stdin, "Content-Length: {}\n\n", body.len()).expect("failed to write header");
    stdin.write_all(&body).expect("failed to write body");
    stdin.flush().expect("failed to flush request");
}

fn send_request_as_json_line(child: &mut Child, request: Value) {
    let body = serde_json::to_vec(&request).expect("request should serialize");
    let stdin = child.stdin.as_mut().expect("missing stdin");
    stdin.write_all(&body).expect("failed to write body");
    stdin.write_all(b"\n").expect("failed to write newline");
    stdin.flush().expect("failed to flush request");
}

fn stop_child(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

fn read_stdout_line(child: &mut Child) -> Receiver<String> {
    let stdout = child.stdout.take().expect("missing stdout");
    let (sender, receiver) = mpsc::channel();

    std::thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        if reader.read_line(&mut line).is_ok() {
            let _ = sender.send(line);
        }
    });

    receiver
}

#[test]
fn mcp_server_responds_to_initialize_without_waiting_for_eof() {
    let (mut child, responses) = spawn_mcp_server();

    send_request(
        &mut child,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {}
        }),
    );

    let response = responses
        .recv_timeout(Duration::from_secs(2))
        .expect("expected initialize response before stdin closes");
    let payload: Value = serde_json::from_str(&response).expect("response should be json");

    stop_child(&mut child);

    assert_eq!(payload["jsonrpc"], "2.0");
    assert_eq!(payload["result"]["serverInfo"]["name"], "ctx-lite-mcp");
}

#[test]
fn mcp_server_handles_multiple_requests_in_one_stdio_session() {
    let (mut child, responses) = spawn_mcp_server();

    send_request(
        &mut child,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {}
        }),
    );
    send_request(
        &mut child,
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list"
        }),
    );

    let initialize = responses
        .recv_timeout(Duration::from_secs(2))
        .expect("expected initialize response");
    let tools = responses
        .recv_timeout(Duration::from_secs(2))
        .expect("expected tools/list response on same session");

    stop_child(&mut child);

    let initialize_payload: Value =
        serde_json::from_str(&initialize).expect("initialize response should be json");
    let tools_payload: Value = serde_json::from_str(&tools).expect("tools response should be json");

    assert_eq!(initialize_payload["jsonrpc"], "2.0");
    assert!(tools_payload["result"]["tools"].is_array());
}

#[test]
fn mcp_server_accepts_lf_only_framing() {
    let (mut child, responses) = spawn_mcp_server();

    send_request_with_lf_only(
        &mut child,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {}
        }),
    );

    let response = responses
        .recv_timeout(Duration::from_secs(2))
        .expect("expected initialize response for LF-only framing");
    let payload: Value = serde_json::from_str(&response).expect("response should be json");

    stop_child(&mut child);

    assert_eq!(payload["jsonrpc"], "2.0");
    assert_eq!(payload["id"], 1);
}

#[test]
fn mcp_server_accepts_json_line_requests() {
    let (mut child, responses) = spawn_mcp_server();

    send_request_as_json_line(
        &mut child,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {}
        }),
    );

    let response = responses
        .recv_timeout(Duration::from_secs(2))
        .expect("expected initialize response for newline-delimited JSON");
    let payload: Value = serde_json::from_str(&response).expect("response should be json");

    stop_child(&mut child);

    assert_eq!(payload["jsonrpc"], "2.0");
    assert_eq!(payload["id"], 1);
}

#[test]
fn mcp_server_replies_with_json_line_for_json_line_requests() {
    let mut child = spawn_raw_mcp_server();
    let lines = read_stdout_line(&mut child);

    send_request_as_json_line(
        &mut child,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {}
        }),
    );

    let line = lines
        .recv_timeout(Duration::from_secs(2))
        .expect("expected newline-delimited initialize response");

    stop_child(&mut child);

    assert!(
        !line.starts_with("Content-Length:"),
        "json-line clients should not receive Content-Length headers"
    );

    let payload: Value = serde_json::from_str(line.trim()).expect("response should be json");
    assert_eq!(payload["jsonrpc"], "2.0");
    assert_eq!(payload["id"], 1);
}
