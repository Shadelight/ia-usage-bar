// Tauri bridge, shared DTO types, and small render helpers used across views.

export type MetricLine =
  | {
      kind: "progress";
      id: string;
      label: string;
      used: number;
      limit: number;
      format: string;
      resetsAt?: string | null;
      resetsInLabel: string;
      windowSecs: number;
      visible: string;
    }
  | { kind: "values"; id: string; label: string; text: string; visible: string }
  | { kind: "badge"; id: string; label: string; text: string; visible: string };

export interface ProviderSnapshot {
  id: string;
  name: string;
  short: string;
  plan: string;
  connected: boolean;
  stale: boolean;
  error: string | null;
  hint: string | null;
  updatedAt: string;
  lines: MetricLine[];
  primaryUtilization: number | null;
}

export interface VendorInfo {
  id: string;
  name: string;
  short: string;
  authKind: string;
  envKey: string | null;
  hint: string;
  needsKey: boolean;
  enabled: boolean;
  detected: boolean;
}

export interface SpendRow {
  id: string;
  name: string;
  label: string;
  usd: number;
}

export interface Dashboard {
  providers: ProviderSnapshot[];
  catalog: VendorInfo[];
  refreshMinutes: number;
  primary: string;
  notifications: boolean;
  showUsageAs: string;
  resetTimes: string;
  nextUpdateInSecs: number;
  spendMonthUsd: number;
  spend: SpendRow[];
  recommendId: string | null;
  recommendName: string | null;
  recommendLeft: number | null;
}

export const $ = (id: string): HTMLElement => document.getElementById(id)!;

export function isTauri(): boolean {
  return !!(window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
}

export async function invokeCmd<T>(
  cmd: string,
  args?: Record<string, unknown>,
): Promise<T | null> {
  if (!isTauri()) return null;
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<T>(cmd, args);
}

export function escapeHtml(s: string): string {
  return s.replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c]!));
}

export const ACCENT: Record<string, string> = {
  anthropic: "#ff7a3d",
  openai: "#34d399",
  openai_admin: "#2dd4bf",
  cursor: "#4ade80",
  copilot: "#a78bfa",
  antigravity: "#fb7185",
  grok: "#e5e7eb",
  supergrok: "#e5e7eb",
  openrouter: "#818cf8",
  zai: "#38bdf8",
  deepseek: "#60a5fa",
  kimi: "#f472b6",
  kilo: "#fbbf24",
  novita: "#c084fc",
  moonshot: "#94a3b8",
  minimax: "#f97316",
  kiro: "#22d3ee",
  nous: "#a3e635",
  opencode_go: "#facc15",
  commandcode: "#fb923c",
  anthropic_api: "#fdba74",
  groq: "#f59e0b",
  windsurf: "#38bdf8",
};

const TAB_NAME: Record<string, string> = {
  anthropic: "Claude",
  openai: "OpenAI",
  openai_admin: "API",
  cursor: "Cursor",
  copilot: "Copilot",
  antigravity: "Gemini",
  grok: "Grok",
  supergrok: "Grok",
  openrouter: "Router",
  zai: "Z.AI",
  deepseek: "DeepSeek",
  kimi: "Kimi",
  kilo: "Kilo",
  novita: "Novita",
  moonshot: "Kimi",
  minimax: "MiniMax",
  kiro: "Kiro",
  nous: "Nous",
  opencode_go: "Go",
  commandcode: "Cmd",
  anthropic_api: "Admin",
  groq: "Groq",
  windsurf: "Windsurf",
};

export function accent(id: string): string {
  return ACCENT[id] || "#60a5fa";
}

export function tabName(id: string, fallback: string): string {
  return TAB_NAME[id] || fallback.split(" ")[0];
}

export function glyph(id: string): string {
  const icons: Record<string, string> = {
    anthropic: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"><path d="M12 3v18M5.2 7.2l13.6 9.6M5.2 16.8l13.6-9.6"/></svg>`,
    openai: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7"><path d="M12 4.5c1.8-1 4-.7 5.4.9 1.2 1.3 1.5 3.2.9 4.8 1.5.9 2.3 2.7 1.9 4.4-.4 1.8-1.9 3.1-3.7 3.4-1 .2-2 .1-2.9-.3-1.8 1-4 .7-5.4-.9-1.2-1.3-1.5-3.2-.9-4.8-1.5-.9-2.3-2.7-1.9-4.4C6 6.8 7.5 5.5 9.3 5.2c.9-.2 1.8-.1 2.7.3z"/></svg>`,
    cursor: `<svg viewBox="0 0 24 24" fill="currentColor"><path d="M12 3 20 8v8l-8 5-8-5V8l8-5zm0 2.3L6.6 8.6v6.8L12 18.7l5.4-3.3V8.6L12 5.3z"/></svg>`,
    copilot: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7"><path d="M5 13v-1a7 7 0 0 1 14 0v1M5 14a3 3 0 0 0-3 3v1h6v-1a3 3 0 0 0-3-3zm14 0a3 3 0 0 1 3 3v1h-6v-1a3 3 0 0 1 3-3zM9 19h6"/></svg>`,
    antigravity: `<svg viewBox="0 0 24 24" fill="currentColor"><path d="M12 3.5 14.2 9l6.3.4-4.9 3.8 1.6 6.1L12 16.2 6.8 19.3 8.4 13.2 3.5 9.4 9.8 9z"/></svg>`,
    grok: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8"><path d="M5 5h6l8 14h-6L5 5zm8 0h6M5 19h6"/></svg>`,
    groq: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8"><circle cx="12" cy="12" r="7"/><path d="M8 12h8"/></svg>`,
    windsurf: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8"><path d="M4 16c3-4 6-6 8-6s5 2 8 6M4 20c3-3 6-4.5 8-4.5S17 17 20 20"/></svg>`,
  };
  return icons[id] || icons.grok;
}
