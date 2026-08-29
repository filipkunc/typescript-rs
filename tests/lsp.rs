use std::{
    io::{BufRead, BufReader, Write},
    process::{Child, ChildStdin, Command, Stdio},
    sync::mpsc::{self, Receiver},
    time::Duration,
};

use serde_json::{Value, json};

struct LspProcess {
    child: Child,
    stdin: ChildStdin,
    messages: Receiver<Value>,
}

impl LspProcess {
    fn start() -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_tsrs"))
            .arg("--lsp")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("start tsrs language server");
        let stdin = child.stdin.take().expect("capture language server stdin");
        let stdout = child.stdout.take().expect("capture language server stdout");
        let (sender, messages) = mpsc::channel();
        std::thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            while let Some(message) = read_message(&mut reader) {
                if sender.send(message).is_err() {
                    break;
                }
            }
        });
        Self {
            child,
            stdin,
            messages,
        }
    }

    fn send(&mut self, message: &Value) {
        let body = serde_json::to_vec(message).expect("serialize LSP message");
        write!(self.stdin, "Content-Length: {}\r\n\r\n", body.len()).expect("write LSP header");
        self.stdin.write_all(&body).expect("write LSP body");
        self.stdin.flush().expect("flush LSP message");
    }

    fn receive(&self) -> Value {
        self.messages
            .recv_timeout(Duration::from_secs(10))
            .expect("receive LSP message")
    }
}

impl Drop for LspProcess {
    fn drop(&mut self) {
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

fn read_message(reader: &mut impl BufRead) -> Option<Value> {
    let mut content_length = None;
    loop {
        let mut header = String::new();
        if reader.read_line(&mut header).ok()? == 0 {
            return None;
        }
        if header == "\r\n" || header == "\n" {
            break;
        }
        if let Some(value) = header.strip_prefix("Content-Length:") {
            content_length = value.trim().parse::<usize>().ok();
        }
    }
    let mut body = vec![0; content_length?];
    reader.read_exact(&mut body).ok()?;
    serde_json::from_slice(&body).ok()
}

#[test]
fn publishes_diagnostics_for_live_document_snapshots() {
    let mut server = LspProcess::start();
    server.send(&json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "processId": null,
            "rootUri": null,
            "capabilities": {}
        }
    }));
    let initialized = server.receive();
    assert_eq!(initialized["id"], 1);
    assert_eq!(initialized["result"]["serverInfo"]["name"], "tsrs");
    assert_eq!(
        initialized["result"]["capabilities"]["positionEncoding"],
        "utf-16"
    );
    assert_eq!(
        initialized["result"]["capabilities"]["textDocumentSync"],
        json!({ "openClose": true, "change": 1 })
    );

    server.send(&json!({
        "jsonrpc": "2.0",
        "method": "initialized",
        "params": {}
    }));
    server.send(&json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
            "textDocument": {
                "uri": "file:///workspace/example.ts",
                "languageId": "typescript",
                "version": 1,
                "text": "let x = \"123\";\nx = 456;"
            }
        }
    }));
    let diagnostics = server.receive();
    assert_eq!(diagnostics["method"], "textDocument/publishDiagnostics");
    assert_eq!(diagnostics["params"]["version"], 1);
    assert_eq!(diagnostics["params"]["diagnostics"][0]["code"], "TS2322");
    assert_eq!(diagnostics["params"]["diagnostics"][0]["source"], "tsrs");
    assert_eq!(
        diagnostics["params"]["diagnostics"][0]["range"]["start"],
        json!({ "line": 1, "character": 0 })
    );

    server.send(&json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didChange",
        "params": {
            "textDocument": {
                "uri": "file:///workspace/example.ts",
                "version": 2
            },
            "contentChanges": [{ "text": "let x = \"123\";\nx = \"456\";" }]
        }
    }));
    let cleared = server.receive();
    assert_eq!(cleared["params"]["version"], 2);
    assert_eq!(cleared["params"]["diagnostics"], json!([]));

    server.send(&json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didClose",
        "params": {
            "textDocument": { "uri": "file:///workspace/example.ts" }
        }
    }));
    let closed = server.receive();
    assert_eq!(closed["params"]["diagnostics"], json!([]));
    assert!(closed["params"]["version"].is_null());

    server.send(&json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "shutdown",
        "params": null
    }));
    assert_eq!(server.receive()["id"], 2);
    server.send(&json!({
        "jsonrpc": "2.0",
        "method": "exit",
        "params": null
    }));
    assert!(server.child.wait().expect("wait for LSP exit").success());
}

#[test]
fn checks_intact_code_while_a_variable_initializer_is_missing() {
    let mut server = LspProcess::start();
    server.send(&json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "processId": null,
            "rootUri": null,
            "capabilities": {}
        }
    }));
    assert_eq!(server.receive()["id"], 1);
    server.send(&json!({
        "jsonrpc": "2.0",
        "method": "initialized",
        "params": {}
    }));
    server.send(&json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
            "textDocument": {
                "uri": "file:///workspace/recovery.ts",
                "languageId": "typescript",
                "version": 1,
                "text": "const broken: number = ;\nconst intact: number = \"wrong\";"
            }
        }
    }));

    let recovered = server.receive();
    assert_eq!(recovered["params"]["version"], 1);
    assert_eq!(
        recovered["params"]["diagnostics"]
            .as_array()
            .expect("diagnostics array")
            .iter()
            .map(|diagnostic| diagnostic["code"].as_str().expect("diagnostic code"))
            .collect::<Vec<_>>(),
        ["TS1109", "TS2322"]
    );

    server.send(&json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didChange",
        "params": {
            "textDocument": {
                "uri": "file:///workspace/recovery.ts",
                "version": 2
            },
            "contentChanges": [{
                "text": "const broken: number = 1;\nconst intact: number = \"wrong\";"
            }]
        }
    }));
    let completed = server.receive();
    assert_eq!(completed["params"]["version"], 2);
    assert_eq!(
        completed["params"]["diagnostics"].as_array().map(Vec::len),
        Some(1)
    );
    assert_eq!(completed["params"]["diagnostics"][0]["code"], "TS2322");

    server.send(&json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "shutdown",
        "params": null
    }));
    assert_eq!(server.receive()["id"], 2);
    server.send(&json!({
        "jsonrpc": "2.0",
        "method": "exit",
        "params": null
    }));
    assert!(server.child.wait().expect("wait for LSP exit").success());
}
