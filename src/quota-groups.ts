import type { UsageQuota } from "./api.ts";
import { summaryPercents } from "./percent.ts";

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

export function groupUsedExact(group: QuotaGroup): number | null {
  const values = group.quotas
    .map((quota) => quota.usedPercent)
    .filter((value): value is number => value != null);
  if (!values.length) return null;
  return Math.max(...values);
}

export function groupSummary(group: QuotaGroup): { used: number; remaining: number } | null {
  const exact = groupUsedExact(group);
  return exact == null ? null : summaryPercents(exact);
}
