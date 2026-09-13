import * as vscode from "vscode";
import { t } from "../i18n";
import { settings } from "../settings";
import { DashboardSnapshot, Provider, UsageQuota } from "../types";
import { availablePercent, escapeMd, formatAge, formatResetLong, usedPercent } from "./format";

/** Keeps the tooltip from becoming "medio metro de altura" when many
 * providers are enabled: past this count, the rest collapse into a single
 * "+N more configured" line instead of rendering in full. */
const MAX_PROVIDERS = 5;

export function tooltip(snapshot: DashboardSnapshot, providers: Provider[], logos: Record<string, string>): vscode.MarkdownString {
  const showAll = settings().showAllMetrics;
  const content = new vscode.MarkdownString(undefined, true);
  content.appendMarkdown(`### ${t("menu.title")}\n\n`);

  const shown = providers.slice(0, MAX_PROVIDERS);
  const overflow = providers.length - shown.length;

  shown.forEach((provider, index) => {
    const name = escapeMd(provider.name);
    const logo = logos[provider.id];
    const badge = logo ? `![${name}](${logo}) **${name}**` : `**${name}**`;
    content.appendMarkdown(`${badge}\n\n`);
    content.appendMarkdown(`*${statusBadge(provider)}*\n\n`);

    const quotas = showAll ? provider.quotas : provider.quotas.filter((quota, i) => i === 0 || (quota.usedPercent ?? 0) > 0);
    for (const quota of quotas) {
      content.appendMarkdown(`**${escapeMd(quota.label)}**\n\n`);
      content.appendMarkdown(`${quotaLine(quota)}\n\n`);
      if (quota.resetInSeconds) content.appendMarkdown(`${t("tooltip.resetsIn", { time: formatResetLong(quota.resetInSeconds) })}\n\n`);
    }
    if (index < shown.length - 1) content.appendMarkdown("---\n\n");
  });

  if (overflow > 0) content.appendMarkdown(`${t("tooltip.moreProviders", { n: overflow })}\n\n`);
  content.appendMarkdown(t("tooltip.updatedAgo", { age: formatAge(snapshot.generatedAt) }));
  return content;
}

/** Used + available always shown together in the tooltip, even though the
 * status bar only ever shows one (per iaUsage.percentageMode) — this is the
 * one place both numbers are visible at a glance. */
function quotaLine(quota: UsageQuota): string {
  const used = usedPercent(quota);
  if (used == null) return "—";
  const available = availablePercent(quota) ?? 0;
  return t("tooltip.usedAvailable", { used: Math.round(used), available: Math.round(available) });
}

function statusBadge(provider: Provider): string {
  if (provider.stale) {
    return `⚠ ${t("tooltip.stale")}${provider.updatedAt ? ` · ${t("common.ago", { age: formatAge(provider.updatedAt) })}` : ""}`;
  }
  return `● ${provider.updatedAt ? t("tooltip.updatedAgo", { age: formatAge(provider.updatedAt) }) : t("tooltip.updatedNow")}`;
}
