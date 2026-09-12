// Pure provider-state derivation: enabled / configured / connection are
// three INDEPENDENT concepts. Enabling a provider never requires prior
// authentication, and no connection outcome may flip `enabled` back.
//
// kept DOM-free on purpose so node --test can cover it without a harness.

import type { ProviderSnapshot, VendorInfo } from "./api.ts";
import { t } from "./i18n.ts";

export type ConnectionState =
  | "connected"
  | "needs_auth"
  | "needs_credential"
  | "needs_permission"
  | "unavailable"
  | "error"
  | "loading"
  | "disabled"
  | "unknown";

export interface DerivedProviderState {
  enabled: boolean;
  /** A usable credential exists (env var, OS keyring, or local login). Never the secret itself. */
  configured: boolean;
  /** Short auth-channel label: OAuth, API key, Local, ... */
  via: string;
  connection: ConnectionState;
  /** Existing `.provider-status-dot` modifier class. */
  dotClass: string;
  /** i18n key for the one-line status subtitle. */
  statusKey: string;
  /** True when the snapshot reports an invalid/expired credential. */
  sessionInvalid: boolean;
}

export function authViaLabel(authKind: string): string {
  switch (authKind) {
    case "oauth":
      return t("viaOAuth");
    case "apikey":
      return t("viaApiKey");
    case "local":
      return t("viaLocal");
    case "mixed":
      return t("viaMixed");
    default:
      return t("viaMixed");
  }
}

function dotFor(connection: ConnectionState): string {
  switch (connection) {
    case "connected":
      return "status-connected";
    case "needs_auth":
    case "needs_credential":
      return "status-needs_auth";
    case "needs_permission":
      return "status-needs_permission";
    case "unavailable":
      return "status-unavailable";
    case "error":
      return "status-error";
    case "disabled":
      return "status-disabled";
    case "loading":
    case "unknown":
    default:
      return "status-loading";
  }
}

export function deriveProviderState(
  vendor: VendorInfo,
  snapshot: ProviderSnapshot | undefined,
  loading: boolean,
): DerivedProviderState {
  const via = authViaLabel(vendor.authKind);
  const configured = vendor.hasCredential || vendor.detected;
  if (!vendor.enabled) {
    return { enabled: false, configured, via, connection: "disabled", dotClass: dotFor("disabled"), statusKey: "statusDisabled", sessionInvalid: false };
  }
  if (!snapshot) {
    if (loading) {
      return { enabled: true, configured, via, connection: "loading", dotClass: dotFor("loading"), statusKey: "statusLoading", sessionInvalid: false };
    }
    if (!configured) {
      return { enabled: true, configured, via, connection: "needs_credential", dotClass: dotFor("needs_credential"), statusKey: "statusNeedKey", sessionInvalid: false };
    }
    return { enabled: true, configured, via, connection: "unknown", dotClass: dotFor("unknown"), statusKey: "statusNoData", sessionInvalid: false };
  }
  switch (snapshot.status) {
    case "connected":
      return { enabled: true, configured, via, connection: "connected", dotClass: dotFor("connected"), statusKey: "statusConnected", sessionInvalid: false };
    case "needs_permission":
      return { enabled: true, configured, via, connection: "needs_permission", dotClass: dotFor("needs_permission"), statusKey: "statusNeedPermission", sessionInvalid: false };
    case "unavailable":
      return { enabled: true, configured, via, connection: "unavailable", dotClass: dotFor("unavailable"), statusKey: "statusUnavailable", sessionInvalid: false };
    case "error":
      return { enabled: true, configured, via, connection: "error", dotClass: dotFor("error"), statusKey: "statusError", sessionInvalid: false };
    case "needs_auth":
    default: {
      const reason = snapshot.statusReason ?? null;
      if (reason === "invalid_credential") {
        // An API key rejection is not an expired "session": OAuth vendors
        // keep the session wording, key vendors name the credential.
        const apiCredential = vendor.needsKey || vendor.authKind === "apikey";
        return { enabled: true, configured, via, connection: "needs_auth", dotClass: dotFor("needs_auth"), statusKey: apiCredential ? "statusCredentialInvalid" : "statusSessionInvalid", sessionInvalid: !apiCredential };
      }
      if (reason === "missing_credential" || reason == null) {
        // Missing credential with no stored key and a key-based vendor:
        // the actionable state is "needs credential", not a generic login.
        if (!vendor.hasCredential && vendor.needsKey) {
          return { enabled: true, configured, via, connection: "needs_credential", dotClass: dotFor("needs_credential"), statusKey: "statusNeedKey", sessionInvalid: false };
        }
        return { enabled: true, configured, via, connection: "needs_auth", dotClass: dotFor("needs_auth"), statusKey: "statusNeedLogin", sessionInvalid: false };
      }
      return { enabled: true, configured, via, connection: "needs_auth", dotClass: dotFor("needs_auth"), statusKey: "statusNeedLogin", sessionInvalid: false };
    }
  }
}

/** One-line subtitle for a provider row. Connected rows append the channel. */
export function formatStatusText(d: DerivedProviderState): string {
  if (d.connection === "connected") return `${t(d.statusKey)} · ${d.via}`;
  return t(d.statusKey);
}

/**
 * The golden input rule, in one place so template and tests share it:
 * editable as soon as the provider is enabled — never gated on
 * configured/connected/credential state, only on a concrete save.
 */
export function isKeyInputDisabled(enabled: boolean, saving: boolean): boolean {
  return !enabled || saving;
}
