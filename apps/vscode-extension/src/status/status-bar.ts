import * as fs from "fs";
import * as vscode from "vscode";
import { t } from "../i18n";
import { settings } from "../settings";
import { DashboardSnapshot } from "../types";
import { buildStatusBarLabel, shouldWarnBackground, visible } from "./format";
import { pickProviders, showMenu } from "./quick-menu";
import { tooltip } from "./tooltip";

const PROVIDER_LOGOS: Record<string, string> = {
  anthropic: "anthropic.svg",
  openai: "openai.svg",
  cursor: "cursor.svg",
  antigravity: "antigravity.svg",
  opencode_go: "opencode_go.svg",
  openrouter: "openrouter.svg",
  deepseek: "deepseek.svg",
  groq: "groq.svg",
  kimi: "kimi.svg",
  kilo: "kilo.svg",
  minimax: "minimax.svg",
  grok: "grok.svg",
  copilot: "copilot.svg",
  windsurf: "windsurf.svg",
  zai: "zai.svg",
  moonshot: "moonshot.svg",
  novita: "novita.svg",
  kiro: "kiro.svg",
  nous: "nous.svg",
};

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
  private refreshing = false;

  constructor(extensionUri: vscode.Uri) {
    this.logos = loadProviderLogos(extensionUri);
    this.item.command = "iaUsage.show";
    this.item.name = t("menu.title");
    this.item.text = `$(loading~spin) ${t("menu.title")}`;
    this.item.tooltip = t("error.waiting");
    this.item.show();
  }

  update(snapshot: DashboardSnapshot): void {
    this.snapshot = snapshot;
    this.render();
  }

  /** Shows a spinner alongside the last known values instead of blanking the
   * status bar while a manual refresh is in flight. */
  setRefreshing(active: boolean): void {
    this.refreshing = active;
    this.render();
  }

  private render(): void {
    if (!this.snapshot) return;
    const config = settings();
    const providers = visible(this.snapshot, config.providers);
    const spinner = this.refreshing ? "$(sync~spin) " : "";
    this.item.text = providers.length
      ? `${spinner}${providers.map((provider) => buildStatusBarLabel(provider, config)).join("  ")}`
      : `${spinner}$(pulse) ${t("menu.title")}`;
    this.item.tooltip = tooltip(this.snapshot, providers, this.logos);
    this.item.backgroundColor = shouldWarnBackground(providers) ? new vscode.ThemeColor("statusBarItem.warningBackground") : undefined;
  }

  showMenu(onRefresh: () => void, onReconnect: () => void): void {
    showMenu(this.snapshot, onRefresh, onReconnect, this.configureProviders);
  }

  configureProviders = (): void => { void pickProviders(this.snapshot); };

  setError(message: string): void {
    this.item.text = `$(warning) ${t("menu.title")}`;
    this.item.tooltip = message;
  }

  dispose(): void { this.item.dispose(); }
}
