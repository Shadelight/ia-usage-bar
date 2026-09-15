import * as vscode from "vscode";
import { t } from "../i18n";
import { settings } from "../settings";
import { DashboardSnapshot, Provider } from "../types";
import { formatResetLong } from "./format";
import { pickProviders } from "./quick-menu";

const config = () => vscode.workspace.getConfiguration("iaUsage");
const setGlobal = (key: string, value: unknown) => config().update(key, value, vscode.ConfigurationTarget.Global);

type QuickAction = "providers" | "appearance" | "primary-metric" | "percentage" | "reset" | "tooltip" | "full-settings";

interface Item extends vscode.QuickPickItem {
  action: QuickAction;
}

/** Entry point for "IA Usage: Customize status bar" (section 9). Everything
 * here writes to Global settings and the status bar re-renders immediately
 * via onDidChangeConfiguration — no settings.json editing, no reload. */
export async function showQuickSettings(snapshot: DashboardSnapshot | undefined): Promise<void> {
  const items: Item[] = [
    { label: t("quickSettings.providers"), action: "providers" },
    { label: t("quickSettings.appearance"), action: "appearance" },
    { label: t("quickSettings.primaryMetric"), action: "primary-metric" },
    { label: t("quickSettings.percentage"), action: "percentage" },
    { label: t("quickSettings.reset"), action: "reset" },
    { label: t("quickSettings.tooltip"), action: "tooltip" },
    { label: t("quickSettings.fullSettings"), action: "full-settings" },
  ];
  const choice = await vscode.window.showQuickPick(items, { title: t("quickSettings.title") });
  if (!choice) return;
  if (choice.action === "providers") await pickProviders(snapshot);
  else if (choice.action === "appearance") await showAppearanceSettings();
  else if (choice.action === "primary-metric") await choosePrimaryMetric(snapshot);
  else if (choice.action === "percentage") await showPercentageSettings();
  else if (choice.action === "reset") await showResetSettings();
  else if (choice.action === "tooltip") await showTooltipInfoSettings();
  else if (choice.action === "full-settings") await vscode.commands.executeCommand("workbench.action.openSettings", "iaUsage");
}

interface RadioItem extends vscode.QuickPickItem {
  value: string;
}

async function pickRadio(title: string, items: RadioItem[]): Promise<string | undefined> {
  const choice = await vscode.window.showQuickPick(items, { title });
  return choice?.value;
}

async function showAppearanceSettings(): Promise<void> {
  const current = settings();
  const density = await pickRadio(t("appearance.title"), [
    { label: `${current.display === "minimal" ? "● " : "○ "}${t("appearance.densityMinimal")}`, value: "minimal" },
    { label: `${current.display === "compact" ? "● " : "○ "}${t("appearance.densityCompact")}`, value: "compact" },
    { label: `${current.display === "full" ? "● " : "○ "}${t("appearance.densityFull")}`, value: "full" },
  ]);
  if (density) await setGlobal("display", density);

  const icons = await pickRadio(t("appearance.icons"), [
    { label: `${current.showProviderIcons ? "● " : "○ "}${t("appearance.iconsOn")}`, value: "on" },
    { label: `${!current.showProviderIcons ? "● " : "○ "}${t("appearance.iconsOff")}`, value: "off" },
  ]);
  if (icons) await setGlobal("showProviderIcons", icons === "on");

  const stale = await pickRadio(t("appearance.stale"), [
    { label: `${current.showStaleIndicator ? "● " : "○ "}${t("appearance.staleOn")}`, value: "on" },
    { label: `${!current.showStaleIndicator ? "● " : "○ "}${t("appearance.staleOff")}`, value: "off" },
  ]);
  if (stale) await setGlobal("showStaleIndicator", stale === "on");
}

async function showPercentageSettings(): Promise<void> {
  const current = settings().percentageMode;
  const mode = await pickRadio(t("percentage.title"), [
    { label: `${current === "used" ? "● " : "○ "}${t("percentage.used")}`, description: t("percentage.usedExample"), value: "used" },
    { label: `${current === "remaining" ? "● " : "○ "}${t("percentage.remaining")}`, description: t("percentage.remainingExample"), value: "remaining" },
  ]);
  if (mode) await setGlobal("percentageMode", mode === "remaining" ? "remaining" : "used");
}

async function showResetSettings(): Promise<void> {
  const current = settings().showResetInStatusBar;
  const mode = await pickRadio(t("reset.title"), [
    { label: `${!current ? "● " : "○ "}${t("reset.hidden")}`, value: "off" },
    { label: `${current ? "● " : "○ "}${t("reset.shown")}`, description: t("reset.shownExample"), value: "on" },
  ]);
  if (mode) await setGlobal("showResetInStatusBar", mode === "on");
}

async function showTooltipInfoSettings(): Promise<void> {
  const current = settings().showAllMetrics;
  const mode = await pickRadio(t("tooltip.info.title"), [
    { label: `${!current ? "● " : "○ "}${t("tooltip.info.primary")}`, description: t("tooltip.info.primaryDesc"), value: "primary" },
    { label: `${current ? "● " : "○ "}${t("tooltip.info.all")}`, description: t("tooltip.info.allDesc"), value: "all" },
  ]);
  if (mode) await setGlobal("showAllMetrics", mode === "all");
}

interface ProviderPickItem extends vscode.QuickPickItem {
  providerId: string;
}

async function choosePrimaryMetric(snapshot: DashboardSnapshot | undefined): Promise<void> {
  const providers = (snapshot?.providers ?? []).filter((provider) => provider.enabled && provider.quotas.length > 1);
  if (providers.length === 0) return;
  const items: ProviderPickItem[] = providers.map((provider) => ({ label: provider.name, providerId: provider.id }));
  const choice = await vscode.window.showQuickPick(items, { title: t("primaryMetric.chooseProvider") });
  const provider = providers.find((p) => p.id === choice?.providerId);
  if (provider) await choosePrimaryMetricForProvider(provider);
}

interface QuotaPickItem extends vscode.QuickPickItem {
  quotaId: string;
}

/** Persists iaUsage.primaryMetric[provider.id] as a stable quota id, never a
 * translated label — labels can change or be localized, ids don't
 * (DashboardSnapshotV1's UsageQuota.id, already stable in the core). */
export async function choosePrimaryMetricForProvider(provider: Provider): Promise<void> {
  if (provider.quotas.length === 0) return;
  const current = settings().primaryMetric[provider.id];
  const items: QuotaPickItem[] = provider.quotas.map((quota, index) => {
    const selected = current ? current === quota.id : index === 0;
    const resets = quota.resetInSeconds ? t("detail.resetsIn", { time: formatResetLong(quota.resetInSeconds) }) : "";
    return {
      label: `${selected ? "● " : "○ "}${quota.label}`,
      description: t("primaryMetric.usedReset", { used: quota.usedPercent?.toFixed(0) ?? "—", resets }),
      quotaId: quota.id,
    };
  });
  const choice = await vscode.window.showQuickPick(items, { title: t("primaryMetric.chooseQuota", { provider: provider.name }) });
  if (!choice) return;
  const primaryMetric = { ...settings().primaryMetric, [provider.id]: choice.quotaId };
  await setGlobal("primaryMetric", primaryMetric);
}
