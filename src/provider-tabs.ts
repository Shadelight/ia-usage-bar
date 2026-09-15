// Pure provider-tab rendering: tab = identity/navigation of the provider.
// No DOM, no assets, no i18n here so `tests/provider-tabs.test.ts` can cover
// it on Node. dash.ts supplies icon/accent/logo HTML from providers.ts.

import { escapeHtml } from "./api.ts";
import type { ProviderSnapshot, VendorInfo } from "./api.ts";

export interface TabVisual {
  icon: string;
  accent: string;
}

/**
 * Short label for the 500–750px band. Prefers the backend `short`; the
 * fallback derives from the (unique, stable) id — never from name initials,
 * which collide ("CC", "CC", "CC").
 */
export function providerShort(vendor: Pick<VendorInfo, "id" | "short">): string {
  if (vendor.short?.trim()) return vendor.short.trim().slice(0, 4);
  return vendor.id
    .replace(/[^a-z0-9]/gi, "")
    .slice(0, 4)
    .toUpperCase();
}

export interface TabInput {
  vendor: VendorInfo;
  snapshot: ProviderSnapshot | undefined;
  active: boolean;
  loading: boolean;
  visual: TabVisual;
  logoHtml: string;
}

function sourceBadgeText(snapshot: ProviderSnapshot | undefined): string {
  switch ((snapshot?.activeSource || "").toLowerCase()) {
    case "oauth":
    case "api":
      return "API";
    case "local-session":
    case "local":
      return "Local";
    default:
      return "";
  }
}

/** Health of the last valid snapshot. Refresh never greys out a live tab. */
export function tabHealthStatus(
  snapshot: ProviderSnapshot | undefined,
  loading: boolean,
): string {
  if (snapshot) {
    if (snapshot.status === "connected") {
      if (snapshot.availability === "partial_limited") return "partial_limited";
      if (snapshot.availability === "blocked") return "error";
      return "connected";
    }
    return snapshot.status;
  }
  return loading ? "loading" : "loading";
}

/** Full tab button HTML: real tab semantics, dual labels, inline plan badge. */
export function tabHtml({ vendor, snapshot, active, loading, visual, logoHtml }: TabInput): string {
  const status = tabHealthStatus(snapshot, loading);
  const plan = snapshot?.plan?.trim() ? snapshot.plan.trim() : "";
  const sourceBadge = sourceBadgeText(snapshot);
  const label = `${vendor.name}${plan ? `, plan ${plan}` : ""}`;
  const title = [vendor.name, plan, sourceBadge].filter(Boolean).join(" · ");
  const badge = active && plan
    ? `<span class="tab-plan">${escapeHtml(plan)}${sourceBadge ? ` · ${escapeHtml(sourceBadge)}` : ""}</span>`
    : "";
  return `<button class="tab${active ? " active" : ""}" role="tab" aria-selected="${active ? "true" : "false"}" tabindex="${active ? "0" : "-1"}" aria-label="${escapeHtml(label)}" title="${escapeHtml(title)}" data-select="${escapeHtml(vendor.id)}" style="--provider-accent:${visual.accent}">
      ${logoHtml}
      <span class="label-col"><span class="label"><span class="label-full">${escapeHtml(vendor.name)}</span><span class="label-short">${escapeHtml(providerShort(vendor))}</span>${badge}</span></span>
      <span class="provider-status-dot status-${status}" title="${escapeHtml(status)}"></span>
    </button>`;
}
