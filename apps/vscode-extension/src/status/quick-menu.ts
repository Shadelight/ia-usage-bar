import * as vscode from "vscode";
import { settings } from "../settings";
import { DashboardSnapshot, Provider } from "../types";
import { percent, providerVisual } from "./provider-visuals";

type MenuAction = "refresh" | "reconnect" | "configure-providers";

interface UsageQuickPickItem extends vscode.QuickPickItem {
  action?: MenuAction;
}

export function showMenu(snapshot: DashboardSnapshot | undefined, onRefresh: () => void, onReconnect: () => void, onConfigureProviders: () => void): void {
  const providers = snapshot ? visible(snapshot, settings().providers) : [];
  const entries: UsageQuickPickItem[] = providers.map((provider) => ({
    label: `$(${providerVisual(provider.id, provider.name).icon}) ${provider.name}  ${percent(provider)}`,
    description: provider.quotas.map((quota) => `${quota.label}: ${quota.usedPercent?.toFixed(0) ?? "—"}%`).join(" · "),
  }));
  entries.push(
    { label: "$(settings-gear) Elegir proveedores…", action: "configure-providers" },
    { label: "$(refresh) Actualizar ahora", description: "Consulta IA Usage una vez", action: "refresh" },
    { label: "$(plug) Reconectar CLI", description: "Reinicia el stream local", action: "reconnect" },
  );
  void vscode.window.showQuickPick(entries, { title: "IA Usage" }).then((choice) => {
    if (choice?.action === "refresh") onRefresh();
    if (choice?.action === "reconnect") onReconnect();
    if (choice?.action === "configure-providers") void pickProviders(snapshot);
  });
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
    void vscode.window.showInformationMessage("IA Usage: aún no hay proveedores activos que mostrar. Actívalos primero en IA Usage Desktop.");
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
    title: "IA Usage: elegir proveedores en la barra",
    canPickMany: true,
  });
  if (choice === undefined) return; // cancelled
  const orderedIds = candidates.map((p) => p.id).filter((id) => choice.some((item) => item.providerId === id));
  await vscode.workspace.getConfiguration("iaUsage").update("providers", orderedIds, vscode.ConfigurationTarget.Global);
}

export function visible(snapshot: DashboardSnapshot, ids: string[]): Provider[] {
  const enabled = snapshot.providers.filter((provider) => provider.enabled);
  const order = new Map(ids.map((id, index) => [id, index]));
  return enabled.filter((provider) => order.has(provider.id)).sort((a, b) => (order.get(a.id) ?? 99) - (order.get(b.id) ?? 99));
}
