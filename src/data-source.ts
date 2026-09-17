// Provider-owned data-source labels for DETAILS.
// Never reuse another provider's source/plan/auth. Node-testable, no DOM.

import type { ProviderSnapshot, VendorInfo } from "./api.ts";
import { t } from "./i18n.ts";
import { deriveProviderState } from "./provider-state.ts";

export interface ProviderDetails {
  providerId: string;
  auth: string;
  source: string;
  plan: string;
}

function fill(template: string, vars: Record<string, string>): string {
  let out = template;
  for (const [key, value] of Object.entries(vars)) out = out.split(`{${key}}`).join(value);
  return out;
}

/**
 * Observed data origin for THIS snapshot only.
 * Unknown/missing source is empty — never inherited from another provider.
 */
export function dataSourceLabel(
  provider: Pick<ProviderSnapshot, "id" | "name" | "activeSource">,
): string {
  const source = (provider.activeSource || "").toLowerCase();
  if (!source) return "";
  const name = (provider.name || "").trim() || provider.id;

  // Only Antigravity's OAuth path actually hits Google Cloud Code APIs.
  if (provider.id === "antigravity" && (source === "oauth" || source === "api")) {
    return t("sourceGoogleApi");
  }

  switch (source) {
    case "local-session":
    case "local":
      return fill(t("sourceNamedLocal"), { name });
    case "oauth":
    case "web-session":
    case "web":
      return name;
    case "api":
      return fill(t("sourceNamedApi"), { name });
    case "cli":
      return t("viaCli");
    default:
      return "";
  }
}

export function assertSnapshotMatchesProvider(
  provider: Pick<ProviderSnapshot, "id">,
  providerId: string,
): void {
  if (provider.id !== providerId) {
    throw new Error(`ProviderSnapshot.id=${provider.id} !== ${providerId}`);
  }
}

export function providerDetails(provider: ProviderSnapshot, vendor: VendorInfo): ProviderDetails {
  assertSnapshotMatchesProvider(provider, vendor.id);
  const derived = deriveProviderState(vendor, provider, false);
  return {
    providerId: provider.id,
    auth: derived.via,
    source: dataSourceLabel(provider),
    plan: provider.plan || "",
  };
}

/** DETAILS contract: only the snapshot of the selected provider. */
export function detailsForSelectedProvider(
  snapshots: ProviderSnapshot[],
  vendors: VendorInfo[],
  selectedId: string,
): ProviderDetails | null {
  const provider = snapshots.find((item) => item.id === selectedId);
  const vendor = vendors.find((item) => item.id === selectedId);
  if (!provider || !vendor) return null;
  return providerDetails(provider, vendor);
}
