import { ChildProcessWithoutNullStreams, execFile, spawn } from "node:child_process";
import * as vscode from "vscode";
import { settings } from "./settings";
import { WatchMessage } from "./types";

export class CliClient implements vscode.Disposable {
  private process?: ChildProcessWithoutNullStreams;
  private buffer = "";
  private restartTimer?: NodeJS.Timeout;
  private stopped = false;

  constructor(private readonly onSnapshot: (message: WatchMessage) => void, private readonly onError: (message: string) => void) {}

  start(): void {
    this.stopped = false;
    this.stopProcess();
    const config = settings();
    const executable = config.cliPath || "iausage";
    try {
      this.process = spawn(executable, ["watch", "--jsonl", "--poll-seconds", String(config.remotePollSeconds)], { windowsHide: true });
    } catch (error) {
      this.onError(`IA Usage: no se pudo iniciar el CLI (${String(error)}).`);
      return;
    }
    this.process.stdout.on("data", (chunk: Buffer) => this.consume(chunk.toString("utf8")));
    this.process.stderr.on("data", (chunk: Buffer) => this.onError(`IA Usage CLI: ${chunk.toString("utf8").trim()}`));
    this.process.on("error", () => this.onError("IA Usage: CLI no encontrado. Configura iaUsage.cliPath o instala iausage en PATH."));
    this.process.on("exit", (code) => {
      this.process = undefined;
      if (!this.stopped) {
        this.onError(`IA Usage: el stream terminó (${code ?? "desconocido"}); reintentando.`);
        this.restartTimer = setTimeout(() => this.start(), 5_000);
      }
    });
  }

  refresh(): Promise<void> {
    const executable = settings().cliPath || "iausage";
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
        this.onError("IA Usage: el CLI emitió una línea JSON inválida.");
      }
    }
  }

  private stopProcess(): void { if (this.process && !this.process.killed) this.process.kill(); this.process = undefined; }
  dispose(): void { this.stopped = true; if (this.restartTimer) clearTimeout(this.restartTimer); this.stopProcess(); }
}
