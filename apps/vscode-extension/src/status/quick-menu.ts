import * as vscode from "vscode";
import { t } from "../i18n";
import { settings } from "../settings";
import { DashboardSnapshot, Provider, UsageQuota } from "../types";
import { formatPercentage, formatResetLong, getPrimaryQuota, visible } from "./format";
import { providerVisual } from "./provider-visuals";
import { choosePrimaryMetricForProvider, showQuickSettings } from "./quick-settings";

type MenuAction = "refresh" | "reconnect" | "configure-providers" | "customize" | "full-settings" | "provider-detail";

interface UsageQuickPickItem extends vscode.QuickPickItem {
  action?: MenuAction;
  providerId?: string;
}

export function showMenu(snapshot: DashboardSnapshot | undefined, onRefresh: () => void, onReconnect: () => void, onConfigureProviders: () => void): void {
  const config = settings();
  const providers = snapshot ? visible(snapshot, config.providers) : [];
  const entries: UsageQuickPickItem[] = providers.map((provider) => ({
    label: `$(${providerVisual(provider.id, provider.name).icon}) ${provider.name}  ${formatPercentage(getPrimaryQuota(provider, config.primaryMetric), config.percentageMode)}`,
    description: quotaSummary(provider),
    action: "provider-detail",
    providerId: provider.id,
  }));
  entries.push(
    { label: t("menu.customize"), action: "customize" },
    { label: t("menu.chooseProviders"), action: "configure-providers" },
    { label: t("menu.refresh"), description: t("menu.refreshDesc"), action: "refresh" },
    { label: t("menu.reconnect"), description: t("menu.reconnectDesc"), action: "reconnect" },
    { label: t("menu.fullSettings"), action: "full-settings" },
  );
  void vscode.window.showQuickPick(entries, { title: t("menu.title") }).then((choice) => {
    if (!choice?.action) return;
    if (choice.action === "refresh") onRefresh();
    else if (choice.action === "reconnect") onReconnect();
    else if (choice.action === "configure-providers") void pickProviders(snapshot);
    else if (choice.action === "customize") void showQuickSettings(snapshot);
    else if (choice.action === "full-settings") void vscode.commands.executeCommand("workbench.action.openSettings", "iaUsage");
    else if (choice.action === "provider-detail" && choice.providerId) {
      const provider = providers.find((p) => p.id === choice.providerId);
      if (provider) void showProviderDetail(provider);
    }
  });
}

function quotaSummary(provider: Provider): string {
  return provider.quotas.map((quota) => `${quota.label}: ${quota.usedPercent?.toFixed(0) ?? "—"}%`).join(" · ");
}

type DetailAction = "choose-metric" | "toggle-visible";

interface DetailQuickPickItem extends vscode.QuickPickItem {
  action?: DetailAction;
}

/** Provider row detail: not decoration, a real drill-down with actions
 * dispatched via `action`, never by matching against a translated label. */
export async function showProviderDetail(provider: Provider): Promise<void> {
  const items: DetailQuickPickItem[] = provider.quotas.map((quota) => ({
    label: quota.label,
    description: quotaLine(quota),
  }));
  items.push(
    { label: t("detail.chooseMetric"), action: "choose-metric" },
    { label: t("detail.toggleVisible"), action: "toggle-visible" },
  );
  const choice = await vscode.window.showQuickPick(items, { title: provider.name });
  if (choice?.action === "choose-metric") await choosePrimaryMetricForProvider(provider);
  else if (choice?.action === "toggle-visible") await toggleProviderVisible(provider.id);
}

function quotaLine(quota: UsageQuota): string {
  const used = quota.usedPercent;
  if (used == null) return "—";
  const available = Math.min(100, Math.max(0, 100 - used));
  const base = t("tooltip.usedAvailable", { used: Math.round(used), available: Math.round(available) });
  return quota.resetInSeconds ? `${base} · ${t("detail.resetsIn", { time: formatResetLong(quota.resetInSeconds) })}` : base;
}

async function toggleProviderVisible(id: string): Promise<void> {
  const config = settings();
  const next = config.providers.includes(id) ? config.providers.filter((p) => p !== id) : [...config.providers, id];
  await vscode.workspace.getConfiguration("iaUsage").update("providers", next, vscode.ConfigurationTarget.Global);
}

interface ProviderPickItem extends vscode.QuickPickItem {
  providerId: string;
}

/** Checkbox picker for `iaUsage.providers`, replacing hand-edited JSON.
 * Candidates come from the last snapshot (so unknown/new core providers show
 * up automatically); falls back to the currently configured ids if the CLI
 * hasn't produced a snapshot yet. */
export async function pickProviders(snapshot: DashboardSnapshot | undefined): Promise<void> {
  const config = settings();
  const candidates: Provider[] = snapshot
    ? snapshot.providers.filter((provider) => provider.enabled)
    : config.providers.map((id) => ({ id, name: id, enabled: true, stale: false, quotas: [] }));
  if (candidates.length === 0) {
    void vscode.window.showInformationMessage(t("providers.noneAvailable"));
    return;
  }
  const selected = new Set(config.providers);
  const items: ProviderPickItem[] = candidates.map((provider) => ({
    label: provider.name,
    description: providerVisual(provider.id, provider.name).short,
    picked: selected.has(provider.id),
    providerId: provider.id,
  }));
  const choice = await vscode.window.showQuickPick(items, {
    title: t("providers.pickTitle"),
    canPickMany: true,
  });
  if (choice === undefined) return; // cancelled
  const orderedIds = candidates.map((p) => p.id).filter((id) => choice.some((item) => item.providerId === id));
  await vscode.workspace.getConfiguration("iaUsage").update("providers", orderedIds, vscode.ConfigurationTarget.Global);
}

/** Sequential single-pick reordering (section 21): no drag/drop WebView,
 * just "pick the next one" until the order is fully specified. */
export async function reorderProviders(snapshot: DashboardSnapshot | undefined): Promise<void> {
  const config = settings();
  const known = new Map((snapshot?.providers ?? []).map((provider) => [provider.id, provider]));
  let remaining: Provider[] = config.providers.map((id) => known.get(id) ?? { id, name: id, enabled: true, stale: false, quotas: [] });
  if (remaining.length < 2) return;
  const ordered: string[] = [];
  while (remaining.length > 1) {
    const items: ProviderPickItem[] = remaining.map((provider) => ({ label: provider.name, providerId: provider.id }));
    const choice = await vscode.window.showQuickPick(items, {
      title: `${t("providers.reorderTitle")} (${ordered.length + 1}/${config.providers.length})`,
    });
    if (!choice) return; // cancelled: leave the existing order untouched
    ordered.push(choice.providerId);
    remaining = remaining.filter((provider) => provider.id !== choice.providerId);
  }
  ordered.push(remaining[0].id);
  await vscode.workspace.getConfiguration("iaUsage").update("providers", ordered, vscode.ConfigurationTarget.Global);
}
