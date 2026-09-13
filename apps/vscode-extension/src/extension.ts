import * as vscode from "vscode";
import { CliClient } from "./cli-client";
import { StatusBar } from "./status-bar";

export function activate(context: vscode.ExtensionContext): void {
  const status = new StatusBar(context.extensionUri);
  const client = new CliClient(
    (message) => status.update(message.snapshot),
    (error) => {
      status.setError(error);
      if (error.startsWith("IA Usage: CLI no encontrado")) {
        void vscode.window.showErrorMessage(error, "Configurar CLI").then((choice) => {
          if (choice === "Configurar CLI") void vscode.commands.executeCommand("workbench.action.openSettings", "iaUsage.cliPath");
        });
      }
    },
  );
  const refresh = () => void client.refresh().catch((error) => vscode.window.showErrorMessage(`IA Usage: no se pudo actualizar (${error.message ?? error}).`));
  context.subscriptions.push(
    status,
    client,
    vscode.commands.registerCommand("iaUsage.show", () => status.showMenu(refresh, () => client.start())),
    vscode.commands.registerCommand("iaUsage.refresh", refresh),
    vscode.commands.registerCommand("iaUsage.restart", () => client.start()),
    vscode.commands.registerCommand("iaUsage.configureCli", () => void vscode.commands.executeCommand("workbench.action.openSettings", "iaUsage.cliPath")),
    vscode.workspace.onDidChangeConfiguration((event) => { if (event.affectsConfiguration("iaUsage")) client.start(); }),
  );
  client.start();
}

export function deactivate(): void {}
