// Spend view + the browser-preview dashboard used when not running in Tauri.

import { $, escapeHtml, tabName } from "../api";
import type { Dashboard, MetricLine, ProviderSnapshot, VendorInfo } from "../api";
import { I18N, lang, t } from "../i18n";

export function renderSpend(dash: Dashboard | null): void {
  if (!dash) return;
  $("spend-title").textContent = t("spend");
  const rows = [...dash.spend].sort((a, b) => b.usd - a.usd);
  if (!rows.length) {
    $("spend-body").innerHTML = `<p class="spend-empty">${t("spendEmpty")}</p>`;
    return;
  }
  $("spend-body").innerHTML = `
    <div class="spend-total">
      <span>${t("spend")}</span>
      <span class="amt">$${dash.spendMonthUsd.toFixed(2)}</span>
    </div>
    ${rows
      .map(
        (r) => `<div class="spend-row">
          <div><div class="name">${escapeHtml(tabName(r.id, r.name))}</div><div class="sub">${escapeHtml(r.label)}</div></div>
          <div>$${r.usd.toFixed(2)}</div>
        </div>`,
      )
      .join("")}
  `;
}

export function previewDashboard(): Dashboard {
  const reset = new Date(Date.now() + 44 * 60 * 1000).toISOString();
  const week = new Date(Date.now() + 75 * 3600 * 1000).toISOString();
  const line = (
    id: string,
    label: string,
    used: number,
    resetsAt: string,
    windowSecs: number,
  ): MetricLine => ({
    kind: "progress",
    id,
    label,
    used,
    limit: 100,
    format: "percent",
    resetsAt,
    resetsInLabel: "",
    windowSecs,
    visible: "always",
  });
  const snap = (
    id: string,
    name: string,
    short: string,
    plan: string,
    a: number,
    b: number,
  ): ProviderSnapshot => ({
    id,
    name,
    short,
    plan,
    connected: true,
    stale: false,
    error: null,
    hint: null,
    updatedAt: new Date().toISOString(),
    lines: [
      line("session", I18N[lang].session, a, reset, 18000),
      line("weekly", I18N[lang].weekly, b, week, 604800),
      ...(id === "cursor"
        ? [line("other", lang === "es" ? "Otros modelos" : "Other models", 10, week, 604800)]
        : []),
    ],
    primaryUtilization: a,
  });
  const catalog = (id: string, name: string, enabled: boolean): VendorInfo => ({
    id,
    name,
    short: id.slice(0, 3).toUpperCase(),
    authKind: "oauth",
    envKey: null,
    hint: "",
    needsKey: false,
    enabled,
    detected: enabled,
  });
  const claude = snap("anthropic", "Claude Code", "CLD", "Max", 88, 12);
  claude.lines.push({ kind: "values", id: "cost_month", label: "Mes", text: "$42.10", visible: "always" });
  return {
    providers: [
      claude,
      snap("openai", "Codex / ChatGPT", "CDX", "Plus", 45, 60),
      snap("cursor", "Cursor", "CUR", "Ultra", 26, 5),
      snap("copilot", "GitHub Copilot", "COP", "Pro", 30, 80),
      snap("antigravity", "Antigravity", "AGY", "Google AI Pro", 55, 25),
    ],
    catalog: [
      catalog("anthropic", "Claude Code", true),
      catalog("openai", "Codex / ChatGPT", true),
      catalog("cursor", "Cursor", true),
      catalog("copilot", "GitHub Copilot", true),
      catalog("antigravity", "Antigravity", true),
      catalog("groq", "Groq", false),
      catalog("windsurf", "Windsurf", false),
    ],
    refreshMinutes: 5,
    primary: "anthropic",
    notifications: true,
    showUsageAs: "used",
    resetTimes: "countdown",
    nextUpdateInSecs: 120,
    spendMonthUsd: 42.1,
    spend: [{ id: "anthropic", name: "Claude Code", label: "Mes", usd: 42.1 }],
    recommendId: "cursor",
    recommendName: "Cursor",
    recommendLeft: 74,
  };
}
