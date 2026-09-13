import { t } from "../i18n";
import type { Settings } from "../settings";
import type { DashboardSnapshot, Provider, UsageQuota } from "../types";
import { providerVisual } from "./provider-visuals";

export function formatResetLong(seconds: number): string {
  if (seconds < 3_600) return `${Math.max(1, Math.floor(seconds / 60))} min`;
  if (seconds < 86_400) {
    const hours = Math.floor(seconds / 3_600);
    const minutes = Math.floor((seconds % 3_600) / 60);
    return minutes > 0 ? `${hours} h ${minutes} min` : `${hours} h`;
  }
  const days = Math.floor(seconds / 86_400);
  const hours = Math.floor((seconds % 86_400) / 3_600);
  return hours > 0 ? `${days} d ${hours} h` : `${days} d`;
}

/** Compact, word-free reset countdown for the status bar: "39m", "1h24m",
 * "3d", "6d7h". Never spells out units with spaces — that's what
 * formatResetLong is for. */
export function formatResetCompact(seconds: number): string {
  if (seconds < 3_600) return `${Math.max(1, Math.floor(seconds / 60))}m`;
  if (seconds < 86_400) {
    const hours = Math.floor(seconds / 3_600);
    const minutes = Math.floor((seconds % 3_600) / 60);
    return minutes > 0 ? `${hours}h${minutes}m` : `${hours}h`;
  }
  const days = Math.floor(seconds / 86_400);
  const hours = Math.floor((seconds % 86_400) / 3_600);
  return hours > 0 ? `${days}d${hours}h` : `${days}d`;
}

export function formatAge(updatedAt: string): string {
  const seconds = Math.max(0, (Date.now() - new Date(updatedAt).getTime()) / 1000);
  if (seconds < 60) return t("common.justNow");
  return formatResetLong(seconds);
}

/** Neutralizes Markdown syntax in CLI-supplied text. isTrusted stays off (no
 * command: links), but escaping still stops a crafted provider/quota name
 * from breaking out of the bold/image markup it's interpolated into. */
export function escapeMd(text: string): string {
  return text.replace(/[\\`*_{}[\]()#+\-.!<>|]/g, (ch) => `\\${ch}`);
}

/** Resolves which quota represents a provider: the one configured via
 * iaUsage.primaryMetric[provider.id] when it still exists, otherwise the
 * provider's first quota. A stale configured id (the core renamed/removed a
 * quota) must never leave the status bar blank. */
export function getPrimaryQuota(provider: Provider, primaryMetric: Record<string, string>): UsageQuota | undefined {
  const configuredId = primaryMetric[provider.id];
  if (configuredId) {
    const match = provider.quotas.find((quota) => quota.id === configuredId);
    if (match) return match;
  }
  return provider.quotas[0];
}

export function usedPercent(quota: UsageQuota | undefined): number | undefined {
  return quota?.usedPercent ?? undefined;
}

export function availablePercent(quota: UsageQuota | undefined): number | undefined {
  const used = usedPercent(quota);
  if (used == null) return undefined;
  return Math.min(100, Math.max(0, 100 - used));
}

/** Single source of truth for "which number goes in the UI": used or
 * available, per iaUsage.percentageMode. Never duplicate this arithmetic in
 * status-bar/tooltip/quick-menu. */
export function formatPercentage(quota: UsageQuota | undefined, mode: Settings["percentageMode"]): string {
  const value = mode === "available" ? availablePercent(quota) : usedPercent(quota);
  return value == null ? "—" : `${Math.round(value)}%`;
}

export function providerLabel(provider: Provider): string {
  return provider.name;
}

/** Enabled providers configured to appear in the status bar, in the order
 * `ids` (iaUsage.providers) specifies — never alphabetical. */
export function visible(snapshot: DashboardSnapshot, ids: string[]): Provider[] {
  const enabled = snapshot.providers.filter((provider) => provider.enabled);
  const order = new Map(ids.map((id, index) => [id, index]));
  return enabled.filter((provider) => order.has(provider.id)).sort((a, b) => (order.get(a.id) ?? 99) - (order.get(b.id) ?? 99));
}

/** statusBarItem.warningBackground is reserved for "every visible provider
 * is stale" — a single stale provider among several fresh ones must not
 * paint the whole bar as an error; it gets the "*" marker instead. */
export function shouldWarnBackground(providers: Provider[]): boolean {
  return providers.length > 0 && providers.every((provider) => provider.stale);
}

/** Builds one provider's status bar segment. Pure string assembly — no
 * vscode API calls — so display density, icons, reset, used/available and
 * the stale marker are each independently unit-testable. */
export function buildStatusBarLabel(provider: Provider, config: Settings): string {
  const visual = providerVisual(provider.id, provider.name);
  const quota = getPrimaryQuota(provider, config.primaryMetric);
  const value = formatPercentage(quota, config.percentageMode);
  const freeSuffix = config.percentageMode === "available" && value !== "—" ? ` ${t("statusbar.free")}` : "";
  const resetSuffix = config.showResetInStatusBar && quota?.resetInSeconds ? ` (${formatResetCompact(quota.resetInSeconds)})` : "";
  const staleMark = config.showStaleIndicator && provider.stale ? "*" : "";
  const iconPart = config.showProviderIcons ? `$(${visual.icon}) ` : "";
  const nameOrCode = config.display === "minimal" ? "" : config.display === "full" ? `${provider.name} ` : `${visual.short} `;
  return `${iconPart}${nameOrCode}${value}${freeSuffix}${resetSuffix}${staleMark}`;
}
