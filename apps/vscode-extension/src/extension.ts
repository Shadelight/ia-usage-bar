import * as vscode from "vscode";
import { CliClient } from "./cli-client";
import { setLang, t } from "./i18n";
import { isRenderOnlyChange, migrateLegacyPercentageMode } from "./settings";
import { reorderProviders } from "./status/quick-menu";
import { showQuickSettings } from "./status/quick-settings";
import { StatusBar } from "./status/status-bar";

export function activate(context: vscode.ExtensionContext): void {
  setLang(vscode.env.language.toLowerCase().startsWith("es") ? "es" : "en");
  const status = new StatusBar(context.extensionUri);
  let lastSnapshot: Parameters<StatusBar["update"]>[0] | undefined;
  const client = new CliClient(
    (message) => {
      lastSnapshot = message.snapshot;
      status.update(message.snapshot);
    },
    (error, kind) => {
      status.setError(error);
      if (kind === "cli-not-found") {
        void vscode.window.showErrorMessage(error, t("error.configureCli")).then((choice) => {
          if (choice === t("error.configureCli")) void vscode.commands.executeCommand("workbench.action.openSettings", "iaUsage.cliPath");
        });
      }
    },
  );
  const refresh = () => {
    status.setRefreshing(true);
    void client.refresh()
      .catch((error) => vscode.window.showErrorMessage(t("error.refreshFailed", { error: error.message ?? String(error) })))
      .finally(() => status.setRefreshing(false));
  };
  context.subscriptions.push(
    status,
    client,
    vscode.commands.registerCommand("iaUsage.show", () => status.showMenu(refresh, () => client.start())),
    vscode.commands.registerCommand("iaUsage.refresh", refresh),
    vscode.commands.registerCommand("iaUsage.restart", () => client.start()),
    vscode.commands.registerCommand("iaUsage.configureCli", () => void vscode.commands.executeCommand("workbench.action.openSettings", "iaUsage.cliPath")),
    vscode.commands.registerCommand("iaUsage.configureProviders", status.configureProviders),
    vscode.commands.registerCommand("iaUsage.customizeStatusBar", () => void showQuickSettings(lastSnapshot)),
    vscode.commands.registerCommand("iaUsage.reorderProviders", () => void reorderProviders(lastSnapshot)),
    vscode.workspace.onDidChangeConfiguration((event) => {
      if (!event.affectsConfiguration("iaUsage")) return;
      // Cosmetic settings (display density, percentage mode, primary metric,
      // etc.) only need a re-render; restarting the CLI process would drop
      // the live watch stream for no reason. Only cliPath/remotePollSeconds
      // actually require reconnecting.
      if (isRenderOnlyChange(event)) {
        if (lastSnapshot) status.update(lastSnapshot);
      } else {
        client.start();
      }
    }),
  );
  void migrateLegacyPercentageMode();
  client.start();
}

export function deactivate(): void {}
