// Tauri bridge, shared DTO types, and small render helpers used across views.

export type MetricLine =
  | {
      kind: "progress";
      id: string;
      label: string;
      used: number;
      remaining: number;
      limit: number;
      format: string;
      resetsAt?: string | null;
      resetsInLabel: string;
      windowSecs: number;
      visible: string;
    }
  | { kind: "values"; id: string; label: string; text: string; visible: string }
  | { kind: "badge"; id: string; label: string; text: string; visible: string };

export type WindowType = "session" | "5h" | "daily" | "weekly" | "monthly" | "credits" | "custom";
export type ResetStatus = "known" | "not_provided" | "not_applicable" | "fetch_failed";
export type UsageSource = "cli" | "oauth" | "api" | "local-session" | "web-session";
export type ProviderStatus = "connected" | "needs_auth" | "needs_permission" | "unavailable" | "error";
export type ProviderStatusReason =
  | "missing_credential"
  | "invalid_credential"
  | "missing_permission"
  | "local_service_unavailable"
  | "network_unavailable"
  | "rate_limited"
  | "parse_failed"
  | "unknown";

export interface UsageQuota {
  id: string;
  label: string;
  windowType: WindowType;
  usedPercent: number | null;
  remainingPercent: number | null;
  usedAmount: number | null;
  limitAmount: number | null;
  unit: "percent" | "credits" | "usd" | "requests" | "tokens" | null;
  resetAt: string | null;
  resetInSeconds: number | null;
  resetStatus: ResetStatus;
  temporaryMultiplier: number | null;
  temporaryExpiresAt: string | null;
  source: UsageSource;
  fetchedAt: string;
  stale: boolean;
}

export interface CreditsSummary {
  remaining: number;
  resetsAvailable: number | null;
}

export interface ProductUsage {
  name: string;
  usedPercent: number;
}

export interface UsageCost {
  today: number | null;
  week: number | null;
  thirtyDays: number | null;
  month: number | null;
}

export interface ProviderUsage {
  id: string;
  name: string;
  short: string;
  plan: string;
  status: ProviderStatus;
  statusReason: ProviderStatusReason | null;
  stale: boolean;
  error: string | null;
  hint: string | null;
  updatedAt: string;
  quotas: UsageQuota[];
  credits: CreditsSummary | null;
  productBreakdown: ProductUsage[];
  cost: UsageCost | null;
  lines: MetricLine[];
  primaryUtilization: number | null;
}

export type ProviderSnapshot = ProviderUsage;

export interface VendorLinks {
  usageUrl: string | null;
  billingUrl: string | null;
  statusUrl: string | null;
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
  links: VendorLinks;
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
  notifyThresholds: number[];
  autostart: boolean;
  alwaysOnTop: boolean;
  compactMode: boolean;
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
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    return await invoke<T>(cmd, args);
  } catch (error) {
    const detail = error instanceof Error ? error.message : String(error);
    window.dispatchEvent(new CustomEvent("app-command-error", { detail }));
    return null;
  }
}

export function escapeHtml(s: string): string {
  return s.replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c]!));
}
