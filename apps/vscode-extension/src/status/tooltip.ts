import * as vscode from "vscode";
import { t } from "../i18n";
import { settings } from "../settings";
import { DashboardSnapshot, Provider, UsageQuota } from "../types";
import {
  escapeMd,
  formatAge,
  formatResetLong,
  groupsFromQuotas,
  isSummaryVisible,
  quotaWindowLabel,
  recommendationCopy,
  summaryPercents,
  usedPercent,
} from "./format";

/** Keeps the tooltip from becoming "medio metro de altura" when many
 * providers are enabled: past this count, the rest collapse into a single
 * "+N more configured" line instead of rendering in full. */
const MAX_PROVIDERS = 5;

export function tooltip(snapshot: DashboardSnapshot, providers: Provider[], logos: Record<string, string>): vscode.MarkdownString {
  const showAll = settings().showAllMetrics;
  const content = new vscode.MarkdownString(undefined, true);
  content.appendMarkdown(`### ${t("menu.title")}\n\n`);
  const recommendation = recommendationCopy(snapshot);
  if (recommendation) {
    const icon = snapshot.recommendation?.severity === "critical" ? "$(error)" : snapshot.recommendation?.severity === "warning" ? "$(warning)" : "$(pass)";
    content.appendMarkdown(`${icon} **${escapeMd(recommendation.title)}**\n\n`);
    content.appendMarkdown(`${escapeMd(recommendation.meta)}\n\n`);
    for (const line of recommendation.detail) content.appendMarkdown(`- ${escapeMd(line)}\n`);
    if (recommendation.detail.length) content.appendMarkdown("\n");
    content.appendMarkdown("---\n\n");
  }

  const shown = providers.slice(0, MAX_PROVIDERS);
  const overflow = providers.length - shown.length;

  shown.forEach((provider, index) => {
    const name = escapeMd(provider.name);
    const logo = logos[provider.id];
    const badge = logo ? `![${name}](${logo}) **${name}**` : `**${name}**`;
    content.appendMarkdown(`${badge}\n\n`);
    content.appendMarkdown(`*${statusBadge(provider)}*\n\n`);

    const groups = groupsFromQuotas(provider.quotas);
    const named = groups.length > 1 || groups.some((group) => group.models.length > 0);
    for (const group of groups) {
      if (named) {
        content.appendMarkdown(`**${escapeMd(group.label)}**\n\n`);
        if (group.models.length) content.appendMarkdown(`*${escapeMd(group.models.join(" · "))}*\n\n`);
      }
      for (const quota of group.quotas) {
        const label = named ? quotaWindowLabel(quota) : quota.label;
        content.appendMarkdown(`**${escapeMd(label)}**\n\n`);
        content.appendMarkdown(`${quotaLine(quota)}\n\n`);
        if (quota.resetInSeconds) content.appendMarkdown(`${t("tooltip.resetsIn", { time: formatResetLong(quota.resetInSeconds) })}\n\n`);
      }
    }
    if (showAll) {
      for (const quota of provider.quotas.filter((item) => !isSummaryVisible(item.visible) && item.id !== "total")) {
        content.appendMarkdown(`**${escapeMd(quota.label)}**\n\n`);
        content.appendMarkdown(`${quotaLine(quota)}\n\n`);
      }
    }
    if (provider.credits) {
      const credits = provider.credits;
      content.appendMarkdown(`${t("tooltip.credits", { balance: Math.floor(credits.remaining) })}${credits.stale ? ` · ${t("tooltip.stale")}` : ""}\n\n`);
      if (credits.resetsAvailable != null) {
        content.appendMarkdown(`${t("tooltip.resetCredits", { n: credits.resetsAvailable })}${credits.resetsStale ? ` · ${t("tooltip.stale")}` : ""}\n\n`);
      }
    }
    if (index < shown.length - 1) content.appendMarkdown("---\n\n");
  });

  if (overflow > 0) content.appendMarkdown(`${t("tooltip.moreProviders", { n: overflow })}\n\n`);
  content.appendMarkdown(t("tooltip.updatedAgo", { age: formatAge(snapshot.generatedAt) }));
  return content;
}

/** Used + remaining always shown together in the tooltip, even though the
 * status bar only ever shows one (per iaUsage.percentageMode) — this is the
 * one place both numbers are visible at a glance. */
function quotaLine(quota: UsageQuota): string {
  const used = usedPercent(quota);
  if (used == null) return "—";
  const summary = summaryPercents(used);
  return t("tooltip.usedAvailable", { used: summary.used, available: summary.remaining });
}

function statusBadge(provider: Provider): string {
  if (provider.stale) {
    return `⚠ ${t("tooltip.stale")}${provider.updatedAt ? ` · ${t("common.ago", { age: formatAge(provider.updatedAt) })}` : ""}`;
  }
  return `● ${provider.updatedAt ? t("tooltip.updatedAgo", { age: formatAge(provider.updatedAt) }) : t("tooltip.updatedNow")}`;
}
