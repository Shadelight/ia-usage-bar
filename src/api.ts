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
export type Availability = "available" | "partial_limited" | "blocked";
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
  groupId?: string | null;
  groupLabel?: string | null;
  models?: string[];
  visible?: string;
}

export interface CreditsSummary {
  remaining: number;
  resetsAvailable: number | null;
}

export interface ProductUsage {
  name: string;
  usedPercent: number;
  parentQuotaId?: string;
  groupId?: string;
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
  availability?: Availability;
  /** Salud del servicio, independiente de la conexión. */
  service?: ServiceHealth;
  /** Fuente que produjo el snapshot actual. */
  activeSource?: UsageSource | null;
  stale: boolean;
  error: string | null;
  hint: string | null;
  /** Latest refresh attempt; unlike updatedAt, this moves even on failure. */
  lastAttemptAt?: string | null;
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
  apiKeyUrl?: string | null;
  signupUrl?: string | null;
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
  /** Origin of an API credential. Metadata only; never contains the secret. */
  credentialSource?: "environment" | "keyring" | "legacy" | null;
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

export interface RecommendCandidate {
  id: string;
  name: string;
  score: number;
  shortHeadroom: number | null;
  longHeadroom: number | null;
  displayLeft: number | null;
  sustainable: boolean | null;
  exhaustInSecs: number | null;
  resetInSecs: number | null;
  hasData: boolean;
  isReserve: boolean;
  excluded?: string | null;
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
  percentageMode?: "used" | "remaining";
  appBootstrapping: boolean;
  refreshing: boolean;
  loadingProviders: string[];
  nextUpdateInSecs: number;
  spendMonthUsd: number;
  spend: SpendRow[];
  recommendId: string | null;
  recommendName: string | null;
  recommendLeft: number | null;
  /** Motor contextual: "stay" | "switch" | "balanced" | "insufficient_data". */
  recommendAction: string;
  /** Código de motivo para i18n (nunca texto libre del backend). */
  recommendReason: string;
  recommendConfidence: number;
  recommendFrom?: string | null;
  recommendScores: RecommendCandidate[];
}

export const $ = (id: string): HTMLElement => document.getElementById(id)!;
export function isTauri(): boolean {
  return !!(window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
}

/**
 * Explicit command outcome. Rust `Result<(), String>` resolves to JavaScript
 * `null` on success, so the payload itself must never be used as a success
 * sentinel.
 */
export type CommandResult<T> =
  | { ok: true; value: T }
  | { ok: false; error?: string };

export async function invokeCmd<T>(
  cmd: string,
  args?: Record<string, unknown>,
): Promise<CommandResult<T>> {
  if (!isTauri()) return { ok: false };
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    return { ok: true, value: await invoke<T>(cmd, args) };
  } catch (error) {
    const detail = error instanceof Error ? error.message : String(error);
    window.dispatchEvent(new CustomEvent("app-command-error", { detail }));
    return { ok: false, error: detail };
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

export interface PairedDeviceDto {
  clientDeviceId: string;
  name: string;
  createdAt: string;
  lastSeenAt: string | null;
}

export interface PendingPairingDto {
  fingerprint: string;
  expiresAt: string;
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
  serverError: string | null;
  lastExport: SyncExportInfo | null;
  pairedDevices: PairedDeviceDto[];
  pendingPairing: PendingPairingDto | null;
}

export interface SyncPairingDto {
  uri: string;
  fingerprint: string;
  host: string;
  port: number;
  qrPngBase64: string;
}

export interface CliInstallStatusDto {
  binaryExists: boolean;
  binaryPath: string;
  pathConfigured: boolean;
  version: string | null;
}
