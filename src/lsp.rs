use std::{
    collections::HashMap,
    sync::{PoisonError, RwLock},
};

use tower_lsp_server::{
    Client, LanguageServer, LspService, Server,
    jsonrpc::Result,
    ls_types::{
        Diagnostic as LspDiagnostic, DiagnosticSeverity, DidChangeTextDocumentParams,
        DidCloseTextDocumentParams, DidOpenTextDocumentParams, InitializeParams, InitializeResult,
        NumberOrString, Position, PositionEncodingKind, Range, ServerCapabilities, ServerInfo,
        TextDocumentSyncCapability, TextDocumentSyncKind, TextDocumentSyncOptions, Uri,
    },
};
use typescript_rs::{Diagnostic, TextRange, check_source};

#[derive(Debug)]
struct Document {
    file_name: String,
    text: String,
    version: i32,
}

#[derive(Debug)]
struct Backend {
    client: Client,
    documents: RwLock<HashMap<Uri, Document>>,
}

impl Backend {
    fn new(client: Client) -> Self {
        Self {
            client,
            documents: RwLock::new(HashMap::new()),
        }
    }

    async fn publish(&self, uri: Uri) {
        let snapshot = {
            let documents = self
                .documents
                .read()
                .unwrap_or_else(PoisonError::into_inner);
            documents.get(&uri).map(|document| {
                (
                    document.file_name.clone(),
                    document.text.clone(),
                    document.version,
                )
            })
        };

        let Some((file_name, text, version)) = snapshot else {
            return;
        };
        let diagnostics = check_source(&file_name, &text)
            .diagnostics
            .iter()
            .map(|diagnostic| to_lsp_diagnostic(diagnostic, &text))
            .collect();
        self.client
            .publish_diagnostics(uri, diagnostics, Some(version))
            .await;
    }
}

impl LanguageServer for Backend {
    async fn initialize(&self, _params: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                position_encoding: Some(PositionEncodingKind::UTF16),
                text_document_sync: Some(TextDocumentSyncCapability::Options(
                    TextDocumentSyncOptions {
                        open_close: Some(true),
                        change: Some(TextDocumentSyncKind::FULL),
                        ..TextDocumentSyncOptions::default()
                    },
                )),
                ..ServerCapabilities::default()
            },
            server_info: Some(ServerInfo {
                name: "tsrs".to_owned(),
                version: Some(env!("CARGO_PKG_VERSION").to_owned()),
            }),
            offset_encoding: None,
        })
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let document = params.text_document;
        let uri = document.uri;
        self.documents
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(
                uri.clone(),
                Document {
                    file_name: uri.to_string(),
                    text: document.text,
                    version: document.version,
                },
            );
        self.publish(uri).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        let version = params.text_document.version;
        let Some(change) = params.content_changes.into_iter().last() else {
            return;
        };
        let changed = {
            let mut documents = self
                .documents
                .write()
                .unwrap_or_else(PoisonError::into_inner);
            let Some(document) = documents.get_mut(&uri) else {
                return;
            };
            if version <= document.version {
                false
            } else {
                document.text = change.text;
                document.version = version;
                true
            }
        };
        if changed {
            self.publish(uri).await;
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;
        self.documents
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(&uri);
        self.client.publish_diagnostics(uri, Vec::new(), None).await;
    }
}

pub(crate) async fn serve_stdio() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let (service, socket) = LspService::new(Backend::new);
    Server::new(stdin, stdout, socket)
        .concurrency_level(1)
        .serve(service)
        .await;
}

fn to_lsp_diagnostic(diagnostic: &Diagnostic, source: &str) -> LspDiagnostic {
    let range = diagnostic.range.map_or_else(
        || Range::new(Position::new(0, 0), Position::new(0, 0)),
        |range| to_lsp_range(source, range),
    );
    LspDiagnostic::new(
        range,
        Some(DiagnosticSeverity::ERROR),
        Some(NumberOrString::String(diagnostic.code.clone())),
        Some("tsrs".to_owned()),
        diagnostic.message.clone(),
        None,
        None,
    )
}

fn to_lsp_range(source: &str, range: TextRange) -> Range {
    Range::new(
        position_at(source, range.start),
        position_at(source, range.end),
    )
}

fn position_at(source: &str, byte_offset: u32) -> Position {
    let offset = usize::try_from(byte_offset)
        .unwrap_or(usize::MAX)
        .min(source.len());
    let offset = floor_char_boundary(source, offset);
    let prefix = &source[..offset];
    let (line, line_prefix) = prefix.rsplit_once('\n').map_or_else(
        || (0, prefix),
        |(_, tail)| (prefix.bytes().filter(|byte| *byte == b'\n').count(), tail),
    );
    let character = line_prefix.encode_utf16().count();
    Position::new(
        u32::try_from(line).unwrap_or(u32::MAX),
        u32::try_from(character).unwrap_or(u32::MAX),
    )
}

fn floor_char_boundary(text: &str, mut index: usize) -> usize {
    while !text.is_char_boundary(index) {
        index -= 1;
    }
    index
}

#[cfg(test)]
mod tests {
    use tower_lsp_server::ls_types::Position;
    use typescript_rs::TextRange;

    use super::{position_at, to_lsp_range};

    #[test]
    fn converts_utf8_offsets_to_utf16_positions() {
        let source = "const value = '😀';\nvalue";

        assert_eq!(position_at(source, 19), Position::new(0, 17));
        assert_eq!(position_at(source, 22), Position::new(1, 0));
    }

    #[test]
    fn converts_diagnostic_ranges_at_the_lsp_boundary() {
        let source = "😀\nvalue";

        assert_eq!(
            to_lsp_range(source, TextRange::new(5, 10)),
            tower_lsp_server::ls_types::Range::new(Position::new(1, 0), Position::new(1, 5))
        );
    }
}
