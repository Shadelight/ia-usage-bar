// Helper logic for provider contextual actions, sanitized diagnosis, icon SVGs, and compact action limits.

import type { ProviderSnapshot, UsageQuota, VendorInfo } from "./api.ts";
import { formatResetRelative } from "./reset-format.ts";

export type IconName =
  | "dashboard"
  | "billing"
  | "status"
  | "docs"
  | "app"
  | "gear"
  | "clipboard"
  | "terminal"
  | "external-link"
  | "chevron-right"
  | "copy"
  | "warning"
  | "refresh"
  | "check";

export function actionIconSvg(name: string, size = 14): string {
  switch (name) {
    case "dashboard":
      return `<svg width="${size}" height="${size}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect width="7" height="9" x="3" y="3" rx="1"/><rect width="7" height="5" x="14" y="3" rx="1"/><rect width="7" height="9" x="14" y="12" rx="1"/><rect width="7" height="5" x="3" y="16" rx="1"/></svg>`;
    case "billing":
      return `<svg width="${size}" height="${size}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect width="20" height="14" x="2" y="5" rx="2"/><line x1="2" x2="22" y1="10" y2="10"/></svg>`;
    case "status":
      return `<svg width="${size}" height="${size}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M22 12h-4l-3 9L9 3l-3 9H2"/></svg>`;
    case "docs":
      return `<svg width="${size}" height="${size}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M4 19.5v-15A2.5 2.5 0 0 1 6.5 2H20v20H6.5a2.5 2.5 0 0 1-2.5-2.5Z"/><path d="M6 6h10"/><path d="M6 10h10"/></svg>`;
    case "app":
      return `<svg width="${size}" height="${size}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect width="18" height="18" x="3" y="3" rx="2"/><path d="M9 3v18"/><path d="m14 9 3 3-3 3"/></svg>`;
    case "gear":
      return `<svg width="${size}" height="${size}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"/></svg>`;
    case "clipboard":
      return `<svg width="${size}" height="${size}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect width="8" height="4" x="8" y="2" rx="1" ry="1"/><path d="M16 4h2a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h2"/></svg>`;
    case "terminal":
      return `<svg width="${size}" height="${size}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="4 17 10 11 4 5"/><line x1="12" x2="20" y1="19" y2="19"/></svg>`;
    case "external-link":
      return `<svg width="${size}" height="${size}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M15 3h6v6"/><path d="M10 14 21 3"/><path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"/></svg>`;
    case "chevron-right":
      return `<svg width="${size}" height="${size}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m9 18 6-6-6-6"/></svg>`;
    case "copy":
      return `<svg width="${size}" height="${size}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect width="14" height="14" x="8" y="8" rx="2" ry="2"/><path d="M4 16c-1.1 0-2-.9-2-2V4c0-1.1.9-2 2-2h10c1.1 0 2 .9 2 2"/></svg>`;
    case "warning":
      return `<svg width="${size}" height="${size}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3Z"/><line x1="12" x2="12" y1="9" y2="13"/><line x1="12" x2="12.01" y1="17" y2="17"/></svg>`;
    case "refresh":
      return `<svg width="${size}" height="${size}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 12a9 9 0 1 1-9-9c2.52 0 4.85.83 6.72 2.24L21 8"/><polyline points="21 3 21 8 16 8"/></svg>`;
    case "check":
      return `<svg width="${size}" height="${size}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"/></svg>`;
    default:
      return "";
  }
}

export interface ProviderActionItem {
  id: string;
  kind: "external" | "configure" | "diagnosis";
  labelKey: string;
  url?: string;
  icon: "dashboard" | "billing" | "status" | "docs" | "app" | "gear" | "clipboard";
  indicator: "external-link" | "chevron-right" | "copy";
}

export function pctOf(quota: UsageQuota): number | null {
  if (quota.usedPercent != null) return Math.max(0, Math.min(100, quota.usedPercent));
  if (quota.usedAmount != null && quota.limitAmount != null && quota.limitAmount > 0) {
    return Math.max(0, Math.min(100, (quota.usedAmount / quota.limitAmount) * 100));
  }
  return null;
}

export function buildSanitizedDiagnosis(provider: ProviderSnapshot, vendor: VendorInfo): string {
  const lines: string[] = [
    vendor.name,
    `Status: ${provider.status}${provider.statusReason ? ` (${provider.statusReason})` : ""}`,
    `Plan: ${provider.plan || "—"}`,
  ];
  for (const quota of provider.quotas) {
    const used = pctOf(quota);
    const reset = quota.resetAt ? formatResetRelative(quota.resetAt, Date.now(), "en") : "";
    const parts = [`${quota.label}: ${used != null ? `${Math.round(used)}% used` : "—"}`];
    if (reset) parts.push(`resets ${reset}`);
    lines.push(`  ${parts.join(" · ")}`);
  }
  if (provider.credits) {
    lines.push(`Credits: ${provider.credits.remaining}${provider.credits.resetsAvailable != null ? ` (resets: ${provider.credits.resetsAvailable})` : ""}`);
  }
  if (provider.cost && (provider.cost.month != null || provider.cost.today != null)) {
    const costParts: string[] = [];
    if (provider.cost.today != null) costParts.push(`Today: $${provider.cost.today.toFixed(2)}`);
    if (provider.cost.month != null) costParts.push(`Month: $${provider.cost.month.toFixed(2)}`);
    lines.push(`Cost: ${costParts.join(", ")}`);
  }
  if (provider.updatedAt) {
    lines.push(`Last update: ${provider.updatedAt}`);
  }
  lines.push("IA Usage: 0.2.0");
  return lines.join("\n");
}

export function getProviderCliCommand(vendor: VendorInfo): string {
  return `iausage usage ${vendor.id}`;
}

export function getProviderActions(vendor: VendorInfo): ProviderActionItem[] {
  const items: ProviderActionItem[] = [];
  const links = vendor.links;

  if (links?.usageUrl) {
    items.push({
      id: "usage",
      kind: "external",
      labelKey: "openUsage",
      url: links.usageUrl,
      icon: "dashboard",
      indicator: "external-link",
    });
  }

  if (links?.billingUrl) {
    items.push({
      id: "billing",
      kind: "external",
      labelKey: "openBilling",
      url: links.billingUrl,
      icon: "billing",
      indicator: "external-link",
    });
  }

  if (links?.statusUrl) {
    items.push({
      id: "status",
      kind: "external",
      labelKey: "openStatus",
      url: links.statusUrl,
      icon: "status",
      indicator: "external-link",
    });
  }

  if (links?.docsUrl) {
    items.push({
      id: "docs",
      kind: "external",
      labelKey: "openDocs",
      url: links.docsUrl,
      icon: "docs",
      indicator: "external-link",
    });
  }

  if (links?.appUrl) {
    items.push({
      id: "app",
      kind: "external",
      labelKey: "openApp",
      url: links.appUrl,
      icon: "app",
      indicator: "external-link",
    });
  }

  items.push({
    id: "configure",
    kind: "configure",
    labelKey: "configureProvider",
    icon: "gear",
    indicator: "chevron-right",
  });

  items.push({
    id: "diagnosis",
    kind: "diagnosis",
    labelKey: "copyDiagnosis",
    icon: "clipboard",
    indicator: "copy",
  });

  return items;
}

export function getCompactQuickActions(vendor: VendorInfo): Array<{ labelKey: string; url?: string; isConfigure?: boolean }> {
  const actions: Array<{ labelKey: string; url?: string; isConfigure?: boolean }> = [];
  if (vendor.links?.usageUrl) {
    actions.push({ labelKey: "panel", url: vendor.links.usageUrl });
  } else if (vendor.links?.billingUrl) {
    actions.push({ labelKey: "billing", url: vendor.links.billingUrl });
  }
  actions.push({ labelKey: "configure", isConfigure: true });
  return actions.slice(0, 2);
}
