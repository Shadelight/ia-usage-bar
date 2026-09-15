// Spend view + the browser-preview dashboard used when not running in Tauri.

import { $, escapeHtml } from "../api";
import type { Dashboard, MetricLine, ProviderSnapshot, UsageQuota, VendorInfo, VendorLinks } from "../api";
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
          <div><div class="name">${escapeHtml(r.name)}</div><div class="sub">${escapeHtml(r.label)}</div></div>
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
    remaining: 100 - used,
    limit: 100,
    format: "percent",
    resetsAt,
    resetsInLabel: "",
    windowSecs,
    visible: "always",
  });
  const quota = (
    id: string,
    label: string,
    usedPercent: number,
    resetAt: string,
    windowType: UsageQuota["windowType"],
  ): UsageQuota => ({
    id,
    label,
    windowType,
    usedPercent,
    remainingPercent: 100 - usedPercent,
    usedAmount: null,
    limitAmount: null,
    unit: "percent",
    resetAt,
    resetInSeconds: Math.max(0, Math.floor((new Date(resetAt).getTime() - Date.now()) / 1_000)),
    resetStatus: "known",
    temporaryMultiplier: null,
    temporaryExpiresAt: null,
    source: "oauth",
    fetchedAt: new Date().toISOString(),
    stale: false,
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
    status: "connected",
    statusReason: null,
    stale: false,
    error: null,
    hint: null,
    updatedAt: new Date().toISOString(),
    quotas: [
      quota(
        id === "openai" ? "5h" : "session",
        id === "openai" ? (lang === "es" ? "5 horas" : "5 hours") : I18N[lang].session,
        a,
        reset,
        id === "openai" ? "5h" : "session",
      ),
      quota("weekly", I18N[lang].weekly, b, week, "weekly"),
      ...(id === "cursor"
        ? [quota("other", lang === "es" ? "Otros modelos" : "Other models", 10, week, "weekly")]
        : []),
    ],
    credits: id === "openai" ? { remaining: 478, resetsAvailable: 1 } : null,
    productBreakdown: id === "anthropic"
      ? [{ name: "Claude Code", usedPercent: 88 }, { name: "Cowork", usedPercent: 12 }]
      : [],
    cost: null,
    lines: [
      line("session", I18N[lang].session, a, reset, 18000),
      line("weekly", I18N[lang].weekly, b, week, 604800),
      ...(id === "cursor"
        ? [line("other", lang === "es" ? "Otros modelos" : "Other models", 10, week, 604800)]
        : []),
    ],
    primaryUtilization: a,
  });
  const noLinks: VendorLinks = { usageUrl: null, billingUrl: null, statusUrl: null };
  const previewLinks: Record<string, VendorLinks> = {
    anthropic: { usageUrl: "https://console.anthropic.com/settings/usage", billingUrl: null, statusUrl: "https://status.anthropic.com" },
    openai: { usageUrl: "https://platform.openai.com/usage", billingUrl: null, statusUrl: "https://status.openai.com" },
    cursor: { usageUrl: "https://cursor.com/dashboard", billingUrl: null, statusUrl: null },
  };
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
    hasCredential: enabled,
    links: previewLinks[id] ?? noLinks,
    strategies: ["oauth"],
    sourcePreference: null,
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
      snap("groq", "Groq", "GRQ", "Developer", 18, 38),
      snap("windsurf", "Windsurf", "WND", "Pro", 62, 44),
    ],
    catalog: [
      catalog("anthropic", "Claude Code", true),
      catalog("openai", "Codex / ChatGPT", true),
      catalog("cursor", "Cursor", true),
      catalog("copilot", "GitHub Copilot", true),
      catalog("antigravity", "Antigravity", true),
      catalog("groq", "Groq", true),
      catalog("windsurf", "Windsurf", true),
    ],
    refreshMinutes: 5,
    refreshAdaptive: true,
    primary: "anthropic",
    notifications: true,
    notifyThresholds: [75, 90, 95],
    autostart: true,
    alwaysOnTop: true,
    compactMode: false,
    appBootstrapping: false,
    refreshing: false,
    loadingProviders: [],
    nextUpdateInSecs: 120,
    spendMonthUsd: 42.1,
    spend: [{ id: "anthropic", name: "Claude Code", label: "Mes", usd: 42.1 }],
    recommendId: "cursor",
    recommendName: "Cursor",
    recommendLeft: 74,
    recommendAction: "stay",
    recommendReason: "sustainable",
    recommendSeverity: "healthy",
    recommendLimitingQuota: null,
    recommendConfidence: 0.85,
    recommendFrom: "anthropic",
    recommendScores: [],
  };
}
