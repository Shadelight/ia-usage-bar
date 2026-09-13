import * as vscode from "vscode";
import { isRenderOnlyChange as isRenderOnlyChangePure } from "./config-change";

export type PercentageMode = "used" | "available";

export interface Settings {
  cliPath: string;
  providers: string[];
  display: "minimal" | "compact" | "full";
  remotePollSeconds: number;
  showAllMetrics: boolean;
  showResetInStatusBar: boolean;
  percentageMode: PercentageMode;
  primaryMetric: Record<string, string>;
  showProviderIcons: boolean;
  showStaleIndicator: boolean;
}

export function settings(): Settings {
  const config = vscode.workspace.getConfiguration("iaUsage");
  const interval = Math.max(15, config.get<number>("remotePollSeconds", 60));
  return {
    cliPath: config.get<string>("cliPath", "").trim(),
    providers: config.get<string[]>("providers", ["anthropic", "openai", "cursor"]),
    display: config.get<"minimal" | "compact" | "full">("display", "compact"),
    remotePollSeconds: interval,
    showAllMetrics: config.get<boolean>("showAllMetrics", false),
    showResetInStatusBar: config.get<boolean>("showResetInStatusBar", false),
    percentageMode: config.get<PercentageMode>("percentageMode", "used"),
    primaryMetric: config.get<Record<string, string>>("primaryMetric", {}),
    showProviderIcons: config.get<boolean>("showProviderIcons", true),
    showStaleIndicator: config.get<boolean>("showStaleIndicator", true),
  };
}

export function isRenderOnlyChange(event: vscode.ConfigurationChangeEvent): boolean {
  return isRenderOnlyChangePure((section) => event.affectsConfiguration(section));
}
