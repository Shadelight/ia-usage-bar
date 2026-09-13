// Minimal fake of the `vscode` module, used only for tests via esbuild's
// --alias:vscode=./test/vscode-mock.ts. Covers exactly the API surface the
// extension actually calls (see `grep vscode\.` across src/) — not a general
// vscode shim. Production bundling still marks vscode --external and never
// touches this file.

export const ConfigurationTarget = { Global: 1, Workspace: 2, WorkspaceFolder: 3 } as const;
export const StatusBarAlignment = { Left: 1, Right: 2 } as const;

export class ThemeColor {
  constructor(public id: string) {}
}

export class MarkdownString {
  value: string;
  isTrusted?: boolean;
  supportThemeIcons?: boolean;
  constructor(value = "", supportThemeIcons = false) {
    this.value = value;
    this.supportThemeIcons = supportThemeIcons;
  }
  appendMarkdown(text: string): this { this.value += text; return this; }
}

export const env = { language: "en-US" };
export function __setLanguage(lang: string): void { env.language = lang; }

interface UpdateCall { section: string; key: string; value: unknown; target: unknown }

let configStore: Record<string, Record<string, unknown>> = {};
let quickPickQueue: unknown[] = [];
let infoMessages: string[] = [];
let errorMessages: string[] = [];
let updateCalls: UpdateCall[] = [];
let executedCommands: { id: string; args: unknown[] }[] = [];

export function __reset(): void {
  configStore = {};
  quickPickQueue = [];
  infoMessages = [];
  errorMessages = [];
  updateCalls = [];
  executedCommands = [];
  env.language = "en-US";
}

export function __setConfig(section: string, values: Record<string, unknown>): void {
  configStore[section] = { ...(configStore[section] ?? {}), ...values };
}

export function __getConfigValue(section: string, key: string): unknown {
  return configStore[section]?.[key];
}

/** Queues the value(s) `showQuickPick` resolves to, one per call, in order. */
export function __queuePick(...values: unknown[]): void { quickPickQueue.push(...values); }
export function __updateCalls(): UpdateCall[] { return updateCalls; }
export function __infoMessages(): string[] { return infoMessages; }
export function __errorMessages(): string[] { return errorMessages; }

function makeConfig(section: string) {
  return {
    get<T>(key: string, def: T): T {
      const values = configStore[section] ?? {};
      return key in values ? (values[key] as T) : def;
    },
    update(key: string, value: unknown, target: unknown): Promise<void> {
      configStore[section] = { ...(configStore[section] ?? {}), [key]: value };
      updateCalls.push({ section, key, value, target });
      return Promise.resolve();
    },
  };
}

export const workspace = {
  getConfiguration: (section: string) => makeConfig(section),
  onDidChangeConfiguration: () => ({ dispose() {} }),
};

export const window = {
  showQuickPick: (_items: unknown, _opts?: unknown) => Promise.resolve(quickPickQueue.shift()),
  showInformationMessage: (message: string) => { infoMessages.push(message); return Promise.resolve(undefined); },
  showErrorMessage: (message: string, ..._items: string[]) => { errorMessages.push(message); return Promise.resolve(undefined); },
  createStatusBarItem: (_alignment?: unknown, _priority?: number) => ({
    text: "",
    tooltip: undefined as unknown,
    command: undefined as unknown,
    name: "",
    backgroundColor: undefined as unknown,
    show() {},
    hide() {},
    dispose() {},
  }),
};

export const commands = {
  registerCommand: (_id: string, _callback: (...args: unknown[]) => unknown) => ({ dispose() {} }),
  executeCommand: (id: string, ...args: unknown[]) => { executedCommands.push({ id, args }); return Promise.resolve(undefined); },
};

export function __executedCommands(): { id: string; args: unknown[] }[] { return executedCommands; }

export const Uri = {
  joinPath: (base: { fsPath: string }, ...segments: string[]) => ({ fsPath: [base.fsPath, ...segments].join("/") }),
};
