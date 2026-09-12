import * as vscode from "vscode";
import { settings } from "./settings";
import { DashboardSnapshot, Provider } from "./types";

export class StatusBar implements vscode.Disposable {
  private readonly item = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Right, 100);
  private snapshot?: DashboardSnapshot;

  constructor() {
    this.item.command = "iaUsage.show";
    this.item.name = "IA Usage";
    this.item.text = "$(pulse) IA Usage";
    this.item.tooltip = "IA Usage: esperando al CLI";
    this.item.show();
  }

  update(snapshot: DashboardSnapshot): void {
    this.snapshot = snapshot;
    const config = settings();
    const providers = visible(snapshot, config.providers);
    this.item.text = providers.length ? `$(pulse) ${providers.map((provider) => label(provider, config.display)).join("  ")}` : "$(pulse) IA Usage";
    this.item.tooltip = tooltip(snapshot, providers);
    this.item.backgroundColor = providers.some((provider) => provider.stale) ? new vscode.ThemeColor("statusBarItem.warningBackground") : undefined;
  }

  showMenu(onRefresh: () => void, onReconnect: () => void): void {
    const providers = this.snapshot ? visible(this.snapshot, settings().providers) : [];
    const entries: vscode.QuickPickItem[] = providers.map((provider) => ({
      label: `${provider.name}  ${percent(provider)}`,
      description: provider.quotas.map((quota) => `${quota.label}: ${quota.usedPercent?.toFixed(0) ?? "—"}%`).join(" · "),
    }));
    entries.push({ label: "$(refresh) Actualizar ahora", description: "Consulta IA Usage una vez" });
    entries.push({ label: "$(plug) Reconectar CLI", description: "Reinicia el stream local" });
    void vscode.window.showQuickPick(entries, { title: "IA Usage" }).then((choice) => {
      if (choice?.label.includes("Actualizar")) onRefresh();
      if (choice?.label.includes("Reconectar")) onReconnect();
    });
  }

  setError(message: string): void { this.item.text = "$(warning) IA Usage"; this.item.tooltip = message; }
  dispose(): void { this.item.dispose(); }
}

function visible(snapshot: DashboardSnapshot, ids: string[]): Provider[] {
  const enabled = snapshot.providers.filter((provider) => provider.enabled);
  const order = new Map(ids.map((id, index) => [id, index]));
  return enabled.filter((provider) => order.has(provider.id)).sort((a, b) => (order.get(a.id) ?? 99) - (order.get(b.id) ?? 99));
}

function label(provider: Provider, mode: "compact" | "full"): string {
  const code = provider.id === "anthropic" ? "C" : provider.id === "openai" ? "O" : provider.id === "cursor" ? "U" : provider.name.slice(0, 2).toUpperCase();
  return mode === "compact" ? `${code}${percent(provider)}` : `${provider.name} ${percent(provider)}`;
}

function percent(provider: Provider): string { return provider.quotas[0]?.usedPercent?.toFixed(0).concat("%") ?? "—"; }

function tooltip(snapshot: DashboardSnapshot, providers: Provider[]): vscode.MarkdownString {
  const content = new vscode.MarkdownString(undefined, true);
  content.appendMarkdown("### IA Usage\n\n");
  for (const provider of providers) {
    content.appendMarkdown(`**${provider.name}**${provider.stale ? " · datos antiguos" : ""}\n\n`);
    for (const quota of provider.quotas) {
      content.appendMarkdown(`${quota.label}: **${quota.usedPercent?.toFixed(0) ?? "—"}% usado**${quota.resetInSeconds ? ` · reinicia ${formatDuration(quota.resetInSeconds)}` : ""}\n\n`);
    }
  }
  content.appendMarkdown(`Actualizado: ${new Date(snapshot.generatedAt).toLocaleString()}`);
  return content;
}

function formatDuration(seconds: number): string {
  if (seconds < 3_600) return `${Math.max(1, Math.floor(seconds / 60))} min`;
  if (seconds < 86_400) return `${Math.floor(seconds / 3_600)} h`;
  return `${Math.floor(seconds / 86_400)} d`;
}
