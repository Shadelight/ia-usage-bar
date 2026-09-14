import type { ProviderSnapshot, ProviderStatus, ProviderStatusReason, VendorInfo } from "./api.ts";
import { t } from "./i18n.ts";

export type ProviderErrorAction =
  | "login"
  | "configure_credentials"
  | "open_settings"
  | "retry"
  | "copy_command"
  | "open_provider"
  | "show_details";

export interface NormalizedProviderError {
  title: string;
  message: string;
  action?: ProviderErrorAction;
  actionLabel?: string;
  technicalDetails?: string;
  missingScopes?: string[];
  severity: "info" | "warning" | "error" | "success";
}

interface CopyEntry {
  title: string;
  message: string;
  severity: NormalizedProviderError["severity"];
  action?: ProviderErrorAction;
  actionLabel?: string;
}

export const STATUS_COPY: Record<Exclude<ProviderStatus, "connected">, Partial<Record<ProviderStatusReason, CopyEntry>>> = {
  needs_auth: {
    missing_credential: { title: "errorMissingCredentialTitle", message: "errorMissingCredentialMessage", severity: "warning", action: "configure_credentials", actionLabel: "actionConfigureCredentials" },
    invalid_credential: { title: "errorInvalidCredentialTitle", message: "errorInvalidCredentialMessage", severity: "warning", action: "login", actionLabel: "actionLogin" },
    oauth_expired: { title: "errorOAuthExpiredTitle", message: "errorOAuthExpiredMessage", severity: "warning", action: "login", actionLabel: "actionLogin" },
  },
  needs_permission: {
    missing_permission: { title: "errorMissingPermissionTitle", message: "errorMissingPermissionMessage", severity: "warning", action: "open_settings", actionLabel: "actionOpenSettings" },
  },
  unavailable: {
    local_service_unavailable: { title: "errorLocalUnavailableTitle", message: "errorLocalUnavailableMessage", severity: "error", action: "retry", actionLabel: "actionRetry" },
    network_unavailable: { title: "errorNetworkUnavailableTitle", message: "errorNetworkUnavailableMessage", severity: "error", action: "retry", actionLabel: "actionRetry" },
  },
  error: {
    parse_failed: { title: "errorParseFailedTitle", message: "errorParseFailedMessage", severity: "error", action: "retry", actionLabel: "actionRetry" },
    unknown: { title: "errorUnknownTitle", message: "errorUnknownMessage", severity: "error", action: "retry", actionLabel: "actionRetry" },
    rate_limited: { title: "errorRateLimitedTitle", message: "errorRateLimitedMessage", severity: "info", action: "retry", actionLabel: "actionRetry" },
  },
};

const FALLBACK_BY_STATUS: Record<Exclude<ProviderStatus, "connected">, CopyEntry> = {
  needs_auth: STATUS_COPY.needs_auth.missing_credential!,
  needs_permission: STATUS_COPY.needs_permission.missing_permission!,
  unavailable: STATUS_COPY.unavailable.network_unavailable!,
  error: STATUS_COPY.error.unknown!,
};

export function sanitizeTechnicalDetails(raw: string): string {
  const sensitiveKey = "(?:authorization|api[-_]?key|[a-z0-9_-]*token[a-z0-9_-]*|cookie|client[-_]?secret)";
  let safe = raw.replace(/\bBearer\s+[^\s,;"'}\]]+/gi, "Bearer [REDACTED]");
  safe = safe.replace(
    new RegExp(`(["']?${sensitiveKey}["']?\\s*:\\s*)"(?:\\\\.|[^"\\\\])*"`, "gi"),
    '$1"[REDACTED]"',
  );
  safe = safe.replace(
    new RegExp(`(${sensitiveKey}\\s*[:=]\\s*)([^\\r\\n,;}]+)`, "gi"),
    "$1[REDACTED]",
  );
  return safe.slice(0, 4_000);
}

// Vendors whose official client the app can launch to sign in again (mirrors
// `provider_login_command` in src-tauri/src/commands.rs). For any other vendor
// a "sign in" button can only fail with "no automatic login": offer a retry
// once the session is renewed in the vendor's own app instead.
const CLIENT_LOGIN_VENDORS = new Set(["anthropic", "openai"]);

export function normalizeProviderError(
  snap: Pick<ProviderSnapshot, "status" | "statusReason" | "error">,
  vendor: VendorInfo,
): NormalizedProviderError | null {
  if (snap.status === "connected") return null;
  const reason = snap.statusReason ?? "unknown";
  // A rejected API key is not an expired OAuth session: point key vendors
  // at replacing the credential instead of signing in again.
  if (
    snap.status === "needs_auth" &&
    reason === "invalid_credential" &&
    (vendor.needsKey || vendor.authKind === "apikey")
  ) {
    const technicalDetails = snap.error ? sanitizeTechnicalDetails(snap.error) : undefined;
    return {
      title: t("errorInvalidApiKeyTitle"),
      message: t("errorInvalidApiKeyMessage"),
      severity: "warning",
      action: "configure_credentials",
      actionLabel: t("actionConfigureCredentials"),
      technicalDetails,
      missingScopes: [],
    };
  }
  const entry = STATUS_COPY[snap.status][reason] ?? FALLBACK_BY_STATUS[snap.status];
  const technicalDetails = snap.error ? sanitizeTechnicalDetails(snap.error) : undefined;
  const canLaunchLogin = entry.action !== "login" || CLIENT_LOGIN_VENDORS.has(vendor.id);
  return {
    title: t(entry.title),
    message: t(entry.message),
    severity: entry.severity,
    action: canLaunchLogin ? entry.action : "retry",
    actionLabel: canLaunchLogin
      ? (entry.actionLabel ? t(entry.actionLabel) : undefined)
      : t("actionRetry"),
    technicalDetails,
    missingScopes: [],
  };
}

export function shouldShowRecoveryToast(
  previousStatus: ProviderStatus | undefined,
  nextStatus: ProviderStatus,
): boolean {
  return previousStatus !== undefined && previousStatus !== "connected" && nextStatus === "connected";
}
