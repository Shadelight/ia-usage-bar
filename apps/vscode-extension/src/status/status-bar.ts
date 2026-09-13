import * as fs from "fs";
import * as vscode from "vscode";
import { settings } from "../settings";
import { DashboardSnapshot, Provider } from "../types";
import { percent, providerVisual } from "./provider-visuals";
import { pickProviders, showMenu, visible } from "./quick-menu";
import { tooltip } from "./tooltip";

const PROVIDER_LOGOS: Record<string, string> = { anthropic: "anthropic.svg", openai: "openai.svg", cursor: "cursor.svg" };

/** Reads each bundled provider logo once and inlines it as a data: URI the tooltip's Markdown can render as `<img>`. */
function loadProviderLogos(extensionUri: vscode.Uri): Record<string, string> {
  const logos: Record<string, string> = {};
  for (const [id, file] of Object.entries(PROVIDER_LOGOS)) {
    try {
      const svg = fs.readFileSync(vscode.Uri.joinPath(extensionUri, "media", "providers", file).fsPath, "utf8");
      logos[id] = `data:image/svg+xml;base64,${Buffer.from(svg, "utf8").toString("base64")}`;
    } catch {
      // Missing/unreadable logo: the tooltip falls back to text-only rows.
    }
  }
  return logos;
}

export class StatusBar implements vscode.Disposable {
  private readonly item = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Right, 100);
  private readonly logos: Record<string, string>;
  private snapshot?: DashboardSnapshot;

  constructor(extensionUri: vscode.Uri) {
    this.logos = loadProviderLogos(extensionUri);
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
    this.item.text = providers.length
      ? `$(${icon}) ${providers.map((provider) => label(provider, config.display)).join("  ")}`
      : "$(pulse) IA Usage";
    this.item.tooltip = tooltip(snapshot, providers, this.logos);
    this.item.backgroundColor = providers.length > 0 && providers.every((provider) => provider.stale) ? new vscode.ThemeColor("statusBarItem.warningBackground") : undefined;
  }

  showMenu(onRefresh: () => void, onReconnect: () => void): void {
    showMenu(this.snapshot, onRefresh, onReconnect, this.configureProviders);
  }

  configureProviders = (): void => { void pickProviders(this.snapshot); };

  setError(message: string): void { this.item.text = "$(warning) IA Usage"; this.item.tooltip = message; }
  dispose(): void { this.item.dispose(); }
}

function label(provider: Provider, mode: "minimal" | "compact" | "full"): string {
  const visual = providerVisual(provider.id, provider.name);
  const value = percent(provider);
  if (mode === "minimal") return `$(${visual.icon}) ${value}`;
  if (mode === "full") return `$(${visual.icon}) ${provider.name} ${value}`;
  return `$(${visual.icon}) ${visual.short} ${value}`;
}
