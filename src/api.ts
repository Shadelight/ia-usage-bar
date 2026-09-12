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
export type UsageSource = "auto" | "cli" | "oauth" | "api" | "local-session" | "web-session" | "web" | "local";
export type DataConfidence = "exact" | "estimated" | "percent_only" | "unknown";
export type ServiceHealth = "operational" | "degraded" | "outage" | "unknown";
export type ProviderStatus = "connected" | "needs_auth" | "needs_permission" | "unavailable" | "error";
export type ProviderStatusReason =
  | "missing_credential"
  | "invalid_credential"
  | "oauth_expired"
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
  confidence?: DataConfidence;
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
  /** "estimated" = derivado de logs locales, no es factura. */
  confidence?: DataConfidence;
}

export interface ProviderUsage {
  id: string;
  name: string;
  short: string;
  plan: string;
  status: ProviderStatus;
  statusReason: ProviderStatusReason | null;
  /** Salud del servicio, independiente de la conexión. */
  service?: ServiceHealth;
  /** Fuente que produjo el snapshot actual. */
  activeSource?: UsageSource | null;
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
  docsUrl?: string | null;
  appUrl?: string | null;
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
  /** Credential present (env/keyring/local login). Boolean only, never the secret. */
  hasCredential: boolean;
  links: VendorLinks;
  /** Estrategias declaradas en orden de preferencia ("oauth"|"cli"|"api"|"web"|"local"). */
  strategies: string[];
  /** Preferencia explícita o null = Automática. */
  sourcePreference: string | null;
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
  refreshAdaptive: boolean;
  primary: string;
  notifications: boolean;
  notifyThresholds: number[];
  autostart: boolean;
  alwaysOnTop: boolean;
  compactMode: boolean;
  appBootstrapping: boolean;
  refreshing: boolean;
  loadingProviders: string[];
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

// M5 sync con el teléfono (comandos sync_* del backend).
export interface SyncExportInfo {
  path: string;
  bytes: number;
}

export interface SyncStatusDto {
  enabled: boolean;
  deviceId: string;
  fingerprint: string;
  exportDir: string;
  hasPassphrase: boolean;
  lan: boolean;
  serverRunning: boolean;
  serverAddr: string;
  lastExport: SyncExportInfo | null;
}

export interface SyncPairingDto {
  uri: string;
  fingerprint: string;
  host: string;
  port: number;
  qrPngBase64: string;
}
