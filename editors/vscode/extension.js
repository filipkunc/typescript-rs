const path = require("node:path");
const vscode = require("vscode");
const { LanguageClient } = require("vscode-languageclient/node");

let client;

async function activate(context) {
  const configuredPath = vscode.workspace
    .getConfiguration("tsrs")
    .get("server.path", "");
  const repositoryRoot = path.resolve(context.extensionPath, "..", "..");
  const command = configuredPath || path.join(repositoryRoot, "target", "debug", "tsrs");
  const workspaceFolder = vscode.workspace.workspaceFolders?.[0];

  client = new LanguageClient(
    "tsrs",
    "tsrs",
    {
      command,
      args: ["--lsp"],
      options: {
        cwd: workspaceFolder?.uri.fsPath || repositoryRoot,
      },
    },
    {
      documentSelector: [
        { scheme: "file", language: "javascript" },
        { scheme: "file", language: "javascriptreact" },
        { scheme: "file", language: "typescript" },
        { scheme: "file", language: "typescriptreact" },
      ],
    },
  );

  await client.start();
}

async function deactivate() {
  if (client) {
    await client.dispose();
    client = undefined;
  }
}

module.exports = { activate, deactivate };
