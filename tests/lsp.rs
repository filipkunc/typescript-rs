use std::{
    io::{BufRead, BufReader, Write},
    process::{Child, ChildStdin, Command, Stdio},
    sync::mpsc::{self, Receiver},
    time::Duration,
};

use serde_json::{Value, json};

#[path = "support/editor_trace.rs"]
mod editor_trace;

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

fn publish_full_change(server: &mut LspProcess, uri: &str, version: i32, text: &str) -> Value {
    server.send(&json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didChange",
        "params": {
            "textDocument": { "uri": uri, "version": version },
            "contentChanges": [{ "text": text }]
        }
    }));
    server.receive()
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
                "text": "const broken =\nconst intact: number = \"wrong\";"
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

#[test]
fn checks_intact_code_while_an_assignment_rhs_is_missing() {
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
                "uri": "file:///workspace/assignment-recovery.ts",
                "languageId": "typescript",
                "version": 1,
                "text": "let target: number = 1;\ntarget = ;\nconst intact: number = \"wrong\";"
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
                "uri": "file:///workspace/assignment-recovery.ts",
                "version": 2
            },
            "contentChanges": [{
                "text": "let target: number = 1;\ntarget = 2;\nconst intact: number = \"wrong\";"
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

#[test]
fn checks_intact_object_properties_while_a_property_value_is_missing() {
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
                "uri": "file:///workspace/object-recovery.ts",
                "languageId": "typescript",
                "version": 1,
                "text": "type Shape = { missing: number; wrong: number };\nconst value: Shape = { missing: , wrong: \"wrong\" };"
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
                "uri": "file:///workspace/object-recovery.ts",
                "version": 2
            },
            "contentChanges": [{
                "text": "type Shape = { missing: number; wrong: number };\nconst value: Shape = { missing: 1, wrong: \"wrong\" };"
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

#[test]
fn checks_intact_array_elements_while_an_operand_is_missing() {
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
                "uri": "file:///workspace/array-recovery.ts",
                "languageId": "typescript",
                "version": 1,
                "text": "let target: number = 1;\nconst values: number[] = [target = , \"wrong\"];"
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
                "uri": "file:///workspace/array-recovery.ts",
                "version": 2
            },
            "contentChanges": [{
                "text": "let target: number = 1;\nconst values: number[] = [target = 1, \"wrong\"];"
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

#[test]
fn checks_intact_arguments_while_a_call_argument_is_missing() {
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
                "uri": "file:///workspace/call-recovery.ts",
                "languageId": "typescript",
                "version": 1,
                "text": "function check(first: number, second: number): void {}\ncheck(, \"wrong\");"
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
        ["TS1135", "TS2345"]
    );

    server.send(&json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didChange",
        "params": {
            "textDocument": {
                "uri": "file:///workspace/call-recovery.ts",
                "version": 2
            },
            "contentChanges": [{
                "text": "function check(first: number, second: number): void {}\ncheck(1, \"wrong\");"
            }]
        }
    }));
    let completed = server.receive();
    assert_eq!(completed["params"]["version"], 2);
    assert_eq!(
        completed["params"]["diagnostics"].as_array().map(Vec::len),
        Some(1)
    );
    assert_eq!(completed["params"]["diagnostics"][0]["code"], "TS2345");

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
fn checks_recovered_lists_while_delimiters_are_missing() {
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

    let recovered_source = concat!(
        "type Shape = { first: number; second: number };\n",
        "const object: Shape = { first: 1 second: \"wrong\" };\n",
        "type Container = { values: number[] };\n",
        "const container: Container = { values: [\"wrong\" };\n",
        "function check(value: number): void {}\n",
        "const calls = [check(\"wrong\"];\n",
        "const intact: number = \"also wrong\";",
    );
    server.send(&json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
            "textDocument": {
                "uri": "file:///workspace/delimiter-recovery.ts",
                "languageId": "typescript",
                "version": 1,
                "text": recovered_source
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
        [
            "TS1005", "TS1005", "TS1005", "TS2322", "TS2322", "TS2345", "TS2322"
        ]
    );

    let completed_source = concat!(
        "type Shape = { first: number; second: number };\n",
        "const object: Shape = { first: 1, second: \"wrong\" };\n",
        "type Container = { values: number[] };\n",
        "const container: Container = { values: [\"wrong\"] };\n",
        "function check(value: number): void {}\n",
        "const calls = [check(\"wrong\")];\n",
        "const intact: number = \"also wrong\";",
    );
    server.send(&json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didChange",
        "params": {
            "textDocument": {
                "uri": "file:///workspace/delimiter-recovery.ts",
                "version": 2
            },
            "contentChanges": [{ "text": completed_source }]
        }
    }));

    let completed = server.receive();
    assert_eq!(completed["params"]["version"], 2);
    assert_eq!(
        completed["params"]["diagnostics"]
            .as_array()
            .expect("diagnostics array")
            .iter()
            .map(|diagnostic| diagnostic["code"].as_str().expect("diagnostic code"))
            .collect::<Vec<_>>(),
        ["TS2322", "TS2322", "TS2345", "TS2322"]
    );

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
fn checks_recovered_functions_while_parameter_delimiters_are_missing() {
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

    let recovered_source = concat!(
        "function format(value: number suffix: string): string { return suffix; }\n",
        "const result: string = format(1, \"ok\");\n",
        "const intact: number = \"wrong\";",
    );
    server.send(&json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
            "textDocument": {
                "uri": "file:///workspace/parameter-recovery.ts",
                "languageId": "typescript",
                "version": 1,
                "text": recovered_source
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
        ["TS1005", "TS2322"]
    );

    let completed_source = concat!(
        "function format(value: number, suffix: string): string { return suffix; }\n",
        "const result: string = format(1, \"ok\");\n",
        "const intact: number = \"wrong\";",
    );
    server.send(&json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didChange",
        "params": {
            "textDocument": {
                "uri": "file:///workspace/parameter-recovery.ts",
                "version": 2
            },
            "contentChanges": [{ "text": completed_source }]
        }
    }));

    let completed = server.receive();
    assert_eq!(completed["params"]["version"], 2);
    assert_eq!(
        completed["params"]["diagnostics"]
            .as_array()
            .expect("diagnostics array")
            .iter()
            .map(|diagnostic| diagnostic["code"].as_str().expect("diagnostic code"))
            .collect::<Vec<_>>(),
        ["TS2322"]
    );

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
fn checks_prior_code_while_a_function_body_closer_is_missing() {
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

    let recovered_source = concat!(
        "const intact: number = \"wrong\";\n",
        "function f(): number {\n",
        "  return 1;\n",
    );
    server.send(&json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
            "textDocument": {
                "uri": "file:///workspace/function-body-recovery.ts",
                "languageId": "typescript",
                "version": 1,
                "text": recovered_source
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
        ["TS1005", "TS2322"]
    );

    server.send(&json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didChange",
        "params": {
            "textDocument": {
                "uri": "file:///workspace/function-body-recovery.ts",
                "version": 2
            },
            "contentChanges": [{ "text": format!("{recovered_source}}}\n") }]
        }
    }));

    let completed = server.receive();
    assert_eq!(completed["params"]["version"], 2);
    assert_eq!(
        completed["params"]["diagnostics"]
            .as_array()
            .expect("diagnostics array")
            .iter()
            .map(|diagnostic| diagnostic["code"].as_str().expect("diagnostic code"))
            .collect::<Vec<_>>(),
        ["TS2322"]
    );

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
fn checks_intact_code_while_a_return_expression_operand_is_missing() {
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

    let recovered_source = concat!(
        "function broken(): number {\n",
        "  return 1 +\n",
        "}\n",
        "const intact: number = \"wrong\";",
    );
    server.send(&json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
            "textDocument": {
                "uri": "file:///workspace/return-recovery.ts",
                "languageId": "typescript",
                "version": 1,
                "text": recovered_source
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

    let completed_source = concat!(
        "function broken(): number {\n",
        "  return 1 + 2;\n",
        "}\n",
        "const intact: number = \"wrong\";",
    );
    server.send(&json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didChange",
        "params": {
            "textDocument": {
                "uri": "file:///workspace/return-recovery.ts",
                "version": 2
            },
            "contentChanges": [{ "text": completed_source }]
        }
    }));

    let completed = server.receive();
    assert_eq!(completed["params"]["version"], 2);
    assert_eq!(
        completed["params"]["diagnostics"]
            .as_array()
            .expect("diagnostics array")
            .iter()
            .map(|diagnostic| diagnostic["code"].as_str().expect("diagnostic code"))
            .collect::<Vec<_>>(),
        ["TS2322"]
    );

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
fn checks_stage_three_interface_member_and_parameter_edits() {
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

    let recovered_source = concat!(
        "interface Box { value: number label: string }\n",
        "declare const box: Box;\n",
        "box.;\n",
        "function broken(, second: number): void {}\n",
        "broken(\"ignored\", 1);\n",
        "const intact: number = \"wrong\";",
    );
    server.send(&json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
            "textDocument": {
                "uri": "file:///workspace/stage-three-recovery.ts",
                "languageId": "typescript",
                "version": 1,
                "text": recovered_source
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
        ["TS1005", "TS1003", "TS1138", "TS2322"]
    );

    let completed_source = concat!(
        "interface Box { value: number; label: string }\n",
        "declare const box: Box;\n",
        "box.value;\n",
        "function broken(first: string, second: number): void {}\n",
        "broken(\"ok\", 1);\n",
        "const intact: number = \"wrong\";",
    );
    server.send(&json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didChange",
        "params": {
            "textDocument": {
                "uri": "file:///workspace/stage-three-recovery.ts",
                "version": 2
            },
            "contentChanges": [{ "text": completed_source }]
        }
    }));

    let completed = server.receive();
    assert_eq!(completed["params"]["version"], 2);
    assert_eq!(
        completed["params"]["diagnostics"]
            .as_array()
            .expect("diagnostics array")
            .iter()
            .map(|diagnostic| diagnostic["code"].as_str().expect("diagnostic code"))
            .collect::<Vec<_>>(),
        ["TS2322"]
    );

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
fn checks_class_members_while_a_separator_is_missing() {
    let mut server = LspProcess::start();
    server.send(&json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": { "processId": null, "rootUri": null, "capabilities": {} }
    }));
    assert_eq!(server.receive()["id"], 1);
    server.send(&json!({
        "jsonrpc": "2.0",
        "method": "initialized",
        "params": {}
    }));

    let recovered_source = concat!(
        "class Box { first: number = 1 second: string = \"ok\"; }\n",
        "const box: Box = new Box();\n",
        "const first: number = box.first;\n",
        "const wrong: number = \"wrong\";",
    );
    server.send(&json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
            "textDocument": {
                "uri": "file:///workspace/class-recovery.ts",
                "languageId": "typescript",
                "version": 1,
                "text": recovered_source
            }
        }
    }));
    let recovered = server.receive();
    assert_eq!(
        recovered["params"]["diagnostics"]
            .as_array()
            .expect("diagnostics array")
            .iter()
            .map(|diagnostic| diagnostic["code"].as_str().expect("diagnostic code"))
            .collect::<Vec<_>>(),
        ["TS1005", "TS2322"]
    );

    let completed_source = recovered_source.replace("1 second", "1; second");
    server.send(&json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didChange",
        "params": {
            "textDocument": {
                "uri": "file:///workspace/class-recovery.ts",
                "version": 2
            },
            "contentChanges": [{ "text": completed_source }]
        }
    }));
    let completed = server.receive();
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

#[test]
fn checks_recovered_types_while_annotations_and_delimiters_are_missing() {
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

    let recovered_source = concat!(
        "type Shape = { unchecked: ; wrong: number };\n",
        "const value: Shape = { unchecked: { anything: true }, wrong: \"wrong\" };\n",
        "type Values = number[;\n",
        "const values: Values = [\"wrong\"];\n",
        "const intact: number = \"also wrong\";",
    );
    server.send(&json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
            "textDocument": {
                "uri": "file:///workspace/type-recovery.ts",
                "languageId": "typescript",
                "version": 1,
                "text": recovered_source
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
        ["TS1110", "TS1005", "TS2322", "TS2322", "TS2322"]
    );

    let completed_source = concat!(
        "type Shape = { unchecked: { anything: boolean }; wrong: number };\n",
        "const value: Shape = { unchecked: { anything: true }, wrong: \"wrong\" };\n",
        "type Values = number[];\n",
        "const values: Values = [\"wrong\"];\n",
        "const intact: number = \"also wrong\";",
    );
    server.send(&json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didChange",
        "params": {
            "textDocument": {
                "uri": "file:///workspace/type-recovery.ts",
                "version": 2
            },
            "contentChanges": [{ "text": completed_source }]
        }
    }));

    let completed = server.receive();
    assert_eq!(completed["params"]["version"], 2);
    assert_eq!(
        completed["params"]["diagnostics"]
            .as_array()
            .expect("diagnostics array")
            .iter()
            .map(|diagnostic| diagnostic["code"].as_str().expect("diagnostic code"))
            .collect::<Vec<_>>(),
        ["TS2322", "TS2322", "TS2322"]
    );

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
fn checks_intact_code_while_source_backed_expressions_are_malformed() {
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
                "uri": "file:///workspace/malformed-expression-recovery.ts",
                "languageId": "typescript",
                "version": 1,
                "text": concat!(
                    "let target: number = 1;\n",
                    "target = ...;\n",
                    "const broken: number = :;\n",
                    "const intact: number = \"wrong\";",
                )
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
        ["TS1109", "TS1109", "TS2322"]
    );

    server.send(&json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didChange",
        "params": {
            "textDocument": {
                "uri": "file:///workspace/malformed-expression-recovery.ts",
                "version": 2
            },
            "contentChanges": [{
                "text": concat!(
                    "let target: number = 1;\n",
                    "target = 2;\n",
                    "const broken: number = 3;\n",
                    "const intact: number = \"wrong\";",
                )
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

#[test]
fn publishes_deterministic_diagnostics_across_deletion_and_repair_sequence() {
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

    let uri = "file:///workspace/deletion-repair-sequence.ts";
    server.send(&json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
            "textDocument": {
                "uri": uri,
                "languageId": "typescript",
                "version": 1,
                "text": concat!(
                    "function check(): void {}\n",
                    "check()\n",
                    "const later: number = \"wrong\";",
                )
            }
        }
    }));
    let clean = server.receive();
    assert_eq!(clean["params"]["version"], 1);
    assert_eq!(diagnostic_codes(&clean), ["TS2322"]);

    let missing_call_closer = publish_full_change(
        &mut server,
        uri,
        2,
        concat!(
            "function check(): void {}\n",
            "check(\n",
            "const later: number = \"wrong\";",
        ),
    );
    assert_eq!(missing_call_closer["params"]["version"], 2);
    assert_eq!(diagnostic_codes(&missing_call_closer), ["TS1135", "TS2322"]);

    let missing_declaration_name = publish_full_change(
        &mut server,
        uri,
        3,
        concat!(
            "function check(): void {}\n",
            "check()\n",
            "const = 1;\n",
            "const later: number = \"wrong\";",
        ),
    );
    assert_eq!(missing_declaration_name["params"]["version"], 3);
    assert_eq!(
        diagnostic_codes(&missing_declaration_name),
        ["TS1134", "TS1134", "TS2322"]
    );

    let repaired = publish_full_change(
        &mut server,
        uri,
        4,
        concat!(
            "function check(): void {}\n",
            "check()\n",
            "const recovered = 1;\n",
            "const later: number = \"wrong\";",
        ),
    );
    assert_eq!(repaired["params"]["version"], 4);
    assert_eq!(diagnostic_codes(&repaired), ["TS2322"]);

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
fn preserves_distant_diagnostics_across_a_large_editor_trace() {
    use editor_trace::{
        COMPLETE_EDIT, MISSING_CALL_CLOSER_EDIT, MISSING_DECLARATION_NAME_EDIT,
        MISSING_PROPERTY_VALUE_EDIT, editor_trace_source,
    };

    let complete = editor_trace_source(COMPLETE_EDIT);
    assert!(
        complete.len() >= 30_000,
        "trace should exercise a larger file"
    );
    assert!(complete.lines().count() >= 1_300);

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

    let uri = "file:///workspace/large-editor-trace.ts";
    server.send(&json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
            "textDocument": {
                "uri": uri,
                "languageId": "typescript",
                "version": 1,
                "text": complete
            }
        }
    }));
    let opened = server.receive();
    assert_trace_diagnostics(&opened, 1, &["TS2322", "TS2322"]);

    let snapshots = [
        (
            2,
            MISSING_PROPERTY_VALUE_EDIT,
            &["TS1109", "TS2322", "TS2322"][..],
        ),
        (3, COMPLETE_EDIT, &["TS2322", "TS2322"][..]),
        (
            4,
            MISSING_CALL_CLOSER_EDIT,
            &["TS1135", "TS2322", "TS2554", "TS2322"][..],
        ),
        (5, COMPLETE_EDIT, &["TS2322", "TS2322"][..]),
        (
            6,
            MISSING_DECLARATION_NAME_EDIT,
            &["TS1134", "TS1134", "TS2322", "TS2322"][..],
        ),
        (7, COMPLETE_EDIT, &["TS2322", "TS2322"][..]),
    ];

    for (version, edit, expected_codes) in snapshots {
        let published = publish_full_change(&mut server, uri, version, &editor_trace_source(edit));
        assert_trace_diagnostics(&published, version, expected_codes);
    }

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

fn assert_trace_diagnostics(message: &Value, version: i32, expected_codes: &[&str]) {
    assert_eq!(message["method"], "textDocument/publishDiagnostics");
    assert_eq!(message["params"]["version"], version);
    assert_eq!(diagnostic_codes(message), expected_codes);

    let diagnostics = message["params"]["diagnostics"]
        .as_array()
        .expect("diagnostics array");
    let sentinel_lines = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic["code"] == "TS2322")
        .map(|diagnostic| {
            diagnostic["range"]["start"]["line"]
                .as_u64()
                .expect("diagnostic line")
        })
        .collect::<Vec<_>>();
    assert_eq!(sentinel_lines.len(), 2);
    assert!(sentinel_lines.first().is_some_and(|line| *line < 25));
    assert!(sentinel_lines.last().is_some_and(|line| *line > 1_300));
}

fn diagnostic_codes(message: &Value) -> Vec<&str> {
    message["params"]["diagnostics"]
        .as_array()
        .expect("diagnostics array")
        .iter()
        .map(|diagnostic| diagnostic["code"].as_str().expect("diagnostic code"))
        .collect()
}
