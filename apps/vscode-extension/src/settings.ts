import * as vscode from "vscode";

export interface Settings {
  cliPath: string;
  providers: string[];
  display: "minimal" | "compact" | "full";
  remotePollSeconds: number;
  showAllMetrics: boolean;
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
  };
}
