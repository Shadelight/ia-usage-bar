import { ChildProcessWithoutNullStreams, execFile, spawn } from "node:child_process";
import { existsSync } from "node:fs";
import { delimiter, join } from "node:path";
import * as vscode from "vscode";
import { t } from "./i18n";
import { settings } from "./settings";
import { WatchMessage } from "./types";

export type CliErrorKind = "cli-not-found" | "generic";

export class CliClient implements vscode.Disposable {
  private process?: ChildProcessWithoutNullStreams;
  private buffer = "";
  private restartTimer?: NodeJS.Timeout;
  private stopped = false;

  constructor(private readonly onSnapshot: (message: WatchMessage) => void, private readonly onError: (message: string, kind: CliErrorKind) => void) {}

  start(): void {
    this.stopped = false;
    this.stopProcess();
    const config = settings();
    const executable = resolveExecutable(config.cliPath);
    if (!executable) {
      this.onError(t("error.cliNotFound"), "cli-not-found");
      return;
    }
    try {
      this.process = spawn(executable, ["watch", "--jsonl", "--poll-seconds", String(config.remotePollSeconds)], { windowsHide: true });
    } catch (error) {
      this.onError(t("error.cliStartFailed", { error: String(error) }), "generic");
      return;
    }
    this.process.stdout.on("data", (chunk: Buffer) => this.consume(chunk.toString("utf8")));
    this.process.stderr.on("data", (chunk: Buffer) => this.onError(`IA Usage CLI: ${chunk.toString("utf8").trim()}`, "generic"));
    this.process.on("error", () => this.onError(t("error.cliNotFound"), "cli-not-found"));
    this.process.on("exit", (code) => {
      this.process = undefined;
      if (!this.stopped) {
        this.onError(t("error.streamEnded", { code: code ?? "?" }), "generic");
        this.restartTimer = setTimeout(() => this.start(), 5_000);
      }
    });
  }

  refresh(): Promise<void> {
    const executable = resolveExecutable(settings().cliPath);
    if (!executable) return Promise.reject(new Error(t("error.cliNotFound")));
    return new Promise((resolve, reject) => execFile(executable, ["refresh"], { windowsHide: true }, (error) => {
      if (error) reject(error); else { this.start(); resolve(); }
    }));
  }

  private consume(data: string): void {
    this.buffer += data;
    const lines = this.buffer.split(/\r?\n/);
    this.buffer = lines.pop() ?? "";
    for (const line of lines) {
      if (!line.trim()) continue;
      try {
        const message = JSON.parse(line) as WatchMessage;
        if (message.type === "snapshot" && message.snapshot?.schemaVersion === 1) this.onSnapshot(message);
      } catch {
        this.onError(t("error.invalidJson"), "generic");
      }
    }
  }

  private stopProcess(): void { if (this.process && !this.process.killed) this.process.kill(); this.process = undefined; }
  dispose(): void { this.stopped = true; if (this.restartTimer) clearTimeout(this.restartTimer); this.stopProcess(); }
}

function resolveExecutable(configuredPath: string): string | undefined {
  if (configuredPath) return existsSync(configuredPath) ? configuredPath : undefined;
  return findOnPath() ?? knownDesktopInstallations().find(existsSync);
}

function findOnPath(): string | undefined {
  const pathValue = process.env.PATH ?? process.env.Path ?? "";
  const names = process.platform === "win32" ? ["iausage.exe", "iausage"] : ["iausage"];
  for (const directory of pathValue.split(delimiter)) {
    if (!directory) continue;
    for (const name of names) {
      const candidate = join(directory.replace(/^"|"$/g, ""), name);
      if (existsSync(candidate)) return candidate;
    }
  }
  return undefined;
}

function knownDesktopInstallations(): string[] {
  if (process.platform !== "win32") return [];
  const roots = [
    process.env.LOCALAPPDATA && join(process.env.LOCALAPPDATA, "Programs", "IA Usage"),
    process.env.LOCALAPPDATA && join(process.env.LOCALAPPDATA, "IA Usage"),
    process.env.ProgramFiles && join(process.env.ProgramFiles, "IA Usage"),
    process.env["ProgramFiles(x86)"] && join(process.env["ProgramFiles(x86)"], "IA Usage"),
    process.env.LOCALAPPDATA && join(process.env.LOCALAPPDATA, "Programs", "IA Usage Bar"),
    process.env.LOCALAPPDATA && join(process.env.LOCALAPPDATA, "IA Usage Bar"),
    process.env.ProgramFiles && join(process.env.ProgramFiles, "IA Usage Bar"),
    process.env["ProgramFiles(x86)"] && join(process.env["ProgramFiles(x86)"], "IA Usage Bar"),
  ].filter((root): root is string => Boolean(root));
  return roots.map((root) => join(root, "resources", "bin", "iausage.exe"));
}
