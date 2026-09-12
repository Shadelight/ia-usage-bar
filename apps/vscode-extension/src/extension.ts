import * as vscode from "vscode";
import { CliClient } from "./cli-client";
import { StatusBar } from "./status-bar";

export function activate(context: vscode.ExtensionContext): void {
  const status = new StatusBar();
  const client = new CliClient(
    (message) => status.update(message.snapshot),
    (error) => status.setError(error),
  );
  const refresh = () => void client.refresh().catch((error) => vscode.window.showErrorMessage(`IA Usage: no se pudo actualizar (${error.message ?? error}).`));
  context.subscriptions.push(
    status,
    client,
    vscode.commands.registerCommand("iaUsage.show", () => status.showMenu(refresh, () => client.start())),
    vscode.commands.registerCommand("iaUsage.refresh", refresh),
    vscode.commands.registerCommand("iaUsage.restart", () => client.start()),
    vscode.workspace.onDidChangeConfiguration((event) => { if (event.affectsConfiguration("iaUsage")) client.start(); }),
  );
  client.start();
}

export function deactivate(): void {}
