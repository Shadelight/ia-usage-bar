import * as vscode from "vscode";
import { settings } from "../settings";
import { DashboardSnapshot, Provider } from "../types";
import { escapeMd, formatAge, formatDuration } from "./format";

export function tooltip(snapshot: DashboardSnapshot, providers: Provider[], logos: Record<string, string>): vscode.MarkdownString {
  const showAll = settings().showAllMetrics;
  const content = new vscode.MarkdownString(undefined, true);
  content.appendMarkdown("### IA Usage\n\n");
  for (const provider of providers) {
    const name = escapeMd(provider.name);
    const logo = logos[provider.id];
    const badge = logo ? `![${name}](${logo}) **${name}**` : `**${name}**`;
    content.appendMarkdown(`${badge}  ${statusBadge(provider)}\n\n`);
    const quotas = showAll ? provider.quotas : provider.quotas.filter((quota, index) => index === 0 || (quota.usedPercent ?? 0) > 0);
    for (const quota of quotas) {
      content.appendMarkdown(`${escapeMd(quota.label)}: **${quota.usedPercent?.toFixed(0) ?? "—"}% usado**${quota.resetInSeconds ? ` · reinicia en ${formatDuration(quota.resetInSeconds)}` : ""}\n\n`);
    }
  }
  content.appendMarkdown(`Actualizado: ${new Date(snapshot.generatedAt).toLocaleString()}`);
  return content;
}

function statusBadge(provider: Provider): string {
  if (provider.stale) return `⚠ Datos antiguos${provider.updatedAt ? ` · hace ${formatAge(provider.updatedAt)}` : ""}`;
  return `● Actualizado${provider.updatedAt ? ` hace ${formatAge(provider.updatedAt)}` : " ahora"}`;
}
