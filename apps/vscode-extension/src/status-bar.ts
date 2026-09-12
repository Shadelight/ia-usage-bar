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
    const icon = providers.length === 0 ? "pulse" : providers.every((provider) => provider.stale) ? "clock" : "check";
    this.item.text = providers.length ? `$(${icon}) ${providers.map((provider) => label(provider, config.display)).join("  ")}` : "$(pulse) IA Usage";
    this.item.tooltip = tooltip(snapshot, providers);
    this.item.backgroundColor = providers.length > 0 && providers.every((provider) => provider.stale) ? new vscode.ThemeColor("statusBarItem.warningBackground") : undefined;
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

const SHORT_CODES: Record<string, string> = { anthropic: "CLD", openai: "CDX", cursor: "CUR" };

function label(provider: Provider, mode: "compact" | "full"): string {
  const code = SHORT_CODES[provider.id] ?? provider.name.slice(0, 3).toUpperCase();
  return mode === "compact" ? `${code} ${percent(provider)}` : `${provider.name} ${percent(provider)}`;
}

function percent(provider: Provider): string { return provider.quotas[0]?.usedPercent?.toFixed(0).concat("%") ?? "—"; }

function tooltip(snapshot: DashboardSnapshot, providers: Provider[]): vscode.MarkdownString {
  const showAll = settings().showAllMetrics;
  const content = new vscode.MarkdownString(undefined, true);
  content.appendMarkdown("### IA Usage\n\n");
  for (const provider of providers) {
    content.appendMarkdown(`**${provider.name}**  ${statusBadge(provider)}\n\n`);
    const quotas = showAll ? provider.quotas : provider.quotas.filter((quota, index) => index === 0 || (quota.usedPercent ?? 0) > 0);
    for (const quota of quotas) {
      content.appendMarkdown(`${quota.label}: **${quota.usedPercent?.toFixed(0) ?? "—"}% usado**${quota.resetInSeconds ? ` · reinicia en ${formatDuration(quota.resetInSeconds)}` : ""}\n\n`);
    }
  }
  content.appendMarkdown(`Actualizado: ${new Date(snapshot.generatedAt).toLocaleString()}`);
  return content;
}

function statusBadge(provider: Provider): string {
  if (provider.stale) return `⚠ Datos antiguos${provider.updatedAt ? ` · hace ${formatAge(provider.updatedAt)}` : ""}`;
  return `● Actualizado${provider.updatedAt ? ` hace ${formatAge(provider.updatedAt)}` : " ahora"}`;
}

function formatAge(updatedAt: string): string {
  const seconds = Math.max(0, (Date.now() - new Date(updatedAt).getTime()) / 1000);
  if (seconds < 60) return "un momento";
  return formatDuration(seconds);
}

function formatDuration(seconds: number): string {
  if (seconds < 3_600) return `${Math.max(1, Math.floor(seconds / 60))} min`;
  if (seconds < 86_400) {
    const hours = Math.floor(seconds / 3_600);
    const minutes = Math.floor((seconds % 3_600) / 60);
    return minutes > 0 ? `${hours} h ${minutes} min` : `${hours} h`;
  }
  const days = Math.floor(seconds / 86_400);
  const hours = Math.floor((seconds % 86_400) / 3_600);
  return hours > 0 ? `${days} d ${hours} h` : `${days} d`;
}
