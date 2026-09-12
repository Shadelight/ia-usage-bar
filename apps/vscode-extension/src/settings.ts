import * as vscode from "vscode";

export interface Settings {
  cliPath: string;
  providers: string[];
  display: "compact" | "full";
  remotePollSeconds: number;
}

export function settings(): Settings {
  const config = vscode.workspace.getConfiguration("iaUsage");
  const interval = Math.max(15, config.get<number>("remotePollSeconds", 60));
  return {
    cliPath: config.get<string>("cliPath", "").trim(),
    providers: config.get<string[]>("providers", ["anthropic", "openai", "cursor"]),
    display: config.get<"compact" | "full">("display", "compact"),
    remotePollSeconds: interval,
  };
}
