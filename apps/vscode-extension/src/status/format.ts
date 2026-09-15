import { t } from "../i18n";
import type { Settings } from "../settings";
import type { DashboardSnapshot, Provider, Recommendation, UsageQuota } from "../types";
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
  const minutes = Math.floor((seconds % 3_600) / 60);
  if (minutes > 0) return `${days} d ${hours} h ${minutes} min`;
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
  return provider.quotas.find((quota) => isSummaryVisible(quota.visible)) ?? provider.quotas[0];
}

export function isSummaryVisible(visible: string | null | undefined): boolean {
  return !visible || visible === "always";
}

export interface QuotaGroup {
  id: string;
  label: string;
  models: string[];
  quotas: UsageQuota[];
}

export function groupsFromQuotas(quotas: UsageQuota[]): QuotaGroup[] {
  const groups: QuotaGroup[] = [];
  for (const quota of quotas) {
    if (!isSummaryVisible(quota.visible) || quota.id === "total") continue;
    const id = quota.groupId || quota.id;
    const existing = groups.find((group) => group.id === id);
    if (existing) {
      if (!existing.label && quota.groupLabel) existing.label = quota.groupLabel;
      if (!existing.models.length && quota.models?.length) existing.models = quota.models.slice();
      existing.quotas.push(quota);
      continue;
    }
    groups.push({
      id,
      label: quota.groupLabel || quota.label,
      models: quota.models?.slice() ?? [],
      quotas: [quota],
    });
  }
  return groups;
}

export function quotaWindowLabel(quota: UsageQuota): string {
  const key = quota.windowType || quota.label;
  if (key === "weekly") return t("quota.weekly");
  if (key === "5h" || key === "five_hour") return t("quota.fiveHour");
  if (key === "session") return t("quota.session");
  return quota.label;
}

export const ROUNDING_VECTORS: ReadonlyArray<[number, number, number]> = [
  [5.48, 5, 95],
  [5.5, 6, 94],
  [14.88, 15, 85],
  [38.2711, 38, 62],
  [99.6, 100, 0],
  [100, 100, 0],
  [-1, 0, 100],
];

export function summaryPercents(usedExact: number): { used: number; remaining: number } {
  const used = !Number.isFinite(usedExact) ? 0 : Math.round(Math.max(0, Math.min(100, usedExact)));
  return { used, remaining: 100 - used };
}

export function usedPercent(quota: UsageQuota | undefined): number | undefined {
  return quota?.usedPercent ?? undefined;
}

export function remainingPercent(quota: UsageQuota | undefined): number | undefined {
  const used = usedPercent(quota);
  if (used == null) return undefined;
  return summaryPercents(used).remaining;
}

/** @deprecated Use remainingPercent. Kept so older tests keep compiling. */
export const availablePercent = remainingPercent;

/** Single source of truth for "which number goes in the UI": used or remaining. */
export function formatPercentage(quota: UsageQuota | undefined, mode: Settings["percentageMode"] | "available"): string {
  const exact = usedPercent(quota);
  if (exact == null) return "—";
  const summary = summaryPercents(exact);
  const value = mode === "remaining" || mode === "available" ? summary.remaining : summary.used;
  return `${value}%`;
}

export function providerLabel(provider: Provider): string {
  return provider.name;
}

export interface RecommendationCopy {
  title: string;
  meta: string;
  detail: string[];
}

export function recommendationCopy(snapshot: DashboardSnapshot): RecommendationCopy | undefined {
  const recommendation = snapshot.recommendation;
  if (!recommendation || recommendation.action === "insufficient_data") return undefined;
  const provider = snapshot.providers.find((item) => item.id === recommendation.fromId);
  const name = (provider?.name ?? recommendation.fromId).replace(/\s+Code$/i, "");
  const quota = recommendation.limitingQuota;
  const titleKey = recommendation.reason === "quota_exhausted"
    ? "recommendation.exhausted"
    : recommendation.severity === "critical"
      ? "recommendation.critical"
      : recommendation.reason === "projected_exhaustion"
        ? "recommendation.warning"
        : "recommendation.healthy";
  const title = t(titleKey, { name });
  let meta = t("recommendation.reachesReset");
  if (quota && recommendation.severity === "critical") {
    meta = quota.resetInSeconds == null
      ? t("recommendation.quotaNoReset", { quota: quota.label, used: Math.round(quota.usedPercent) })
      : t("recommendation.quota", {
          quota: quota.label,
          used: Math.round(quota.usedPercent),
          reset: formatResetLong(quota.resetInSeconds),
        });
  } else if (quota?.exhaustsBeforeResetSeconds != null) {
    meta = t("recommendation.projected", { time: formatResetLong(quota.exhaustsBeforeResetSeconds) });
  }
  const detail: string[] = [];
  if (quota?.expectedUsedPercent != null) detail.push(t("recommendation.expected", { value: Math.round(quota.expectedUsedPercent) }));
  if (quota) detail.push(t("recommendation.actual", { value: Math.round(quota.usedPercent) }));
  if (quota?.deltaPercent != null && quota.deltaPercent > 0) detail.push(t("recommendation.delta", { value: Math.round(quota.deltaPercent) }));
  if (recommendation.action === "switch" && recommendation.toName) {
    detail.push(t("recommendation.alternative", { name: recommendation.toName }));
  }
  return { title, meta, detail };
}

export function recommendationBackground(recommendation: Recommendation | null | undefined): "error" | "warning" | undefined {
  if (recommendation?.severity === "critical") return "error";
  if (recommendation?.severity === "warning") return "warning";
  return undefined;
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
  const remainingMode = config.percentageMode === "remaining";
  const freeSuffix = remainingMode && value !== "—" ? ` ${t("statusbar.free")}` : "";
  const resetSuffix = config.showResetInStatusBar && quota?.resetInSeconds ? ` (${formatResetCompact(quota.resetInSeconds)})` : "";
  const staleMark = config.showStaleIndicator && provider.stale ? "*" : "";
  const iconPart = config.showProviderIcons ? `$(${visual.icon}) ` : "";
  const nameOrCode = config.display === "minimal" ? "" : config.display === "full" ? `${provider.name} ` : `${visual.short} `;
  return `${iconPart}${nameOrCode}${value}${freeSuffix}${resetSuffix}${staleMark}`;
}
