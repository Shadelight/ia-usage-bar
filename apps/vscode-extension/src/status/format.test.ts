import assert from "node:assert/strict";
import { test } from "node:test";
import { setLang } from "../i18n";
import type { Settings } from "../settings";
import type { DashboardSnapshot, Provider, UsageQuota } from "../types";
import {
  availablePercent,
  buildStatusBarLabel,
  formatPercentage,
  formatResetCompact,
  formatResetLong,
  getPrimaryQuota,
  recommendationBackground,
  recommendationCopy,
  ROUNDING_VECTORS,
  shouldWarnBackground,
  summaryPercents,
  usedPercent,
  visible,
} from "./format";

setLang("en");

function quota(overrides: Partial<UsageQuota> = {}): UsageQuota {
  return { id: "session", label: "Session", usedPercent: 21, resetInSeconds: null, stale: false, ...overrides };
}

function provider(overrides: Partial<Provider> = {}): Provider {
  return { id: "openai", name: "Codex / ChatGPT", enabled: true, stale: false, quotas: [quota()], ...overrides };
}

function config(overrides: Partial<Settings> = {}): Settings {
  return {
    cliPath: "",
    providers: ["openai"],
    display: "compact",
    remotePollSeconds: 60,
    showAllMetrics: false,
    showResetInStatusBar: false,
    percentageMode: "used",
    primaryMetric: {},
    showProviderIcons: true,
    showStaleIndicator: true,
    ...overrides,
  };
}

test("formatResetCompact never spells out words or adds spaces", () => {
  assert.equal(formatResetCompact(59), "1m");
  assert.equal(formatResetCompact(39 * 60), "39m");
  assert.equal(formatResetCompact(84 * 60), "1h24m");
  assert.equal(formatResetCompact(3 * 3600), "3h");
  assert.equal(formatResetCompact(3 * 86400), "3d");
  assert.equal(formatResetCompact(6 * 86400 + 7 * 3600), "6d7h");
});

test("formatResetLong spells out units for the tooltip", () => {
  assert.equal(formatResetLong(39 * 60), "39 min");
  assert.equal(formatResetLong(4 * 3600 + 31 * 60), "4 h 31 min");
  assert.equal(formatResetLong(3 * 86400), "3 d");
  assert.equal(formatResetLong(3 * 86400 + 16 * 3600 + 5 * 60), "3 d 16 h 5 min");
});

test("summaryPercents matches the cross-platform rounding contract", () => {
  for (const [exact, used, remaining] of ROUNDING_VECTORS) {
    assert.deepEqual(summaryPercents(exact), { used, remaining }, `usedExact=${exact}`);
  }
});

test("getPrimaryQuota uses the configured quota id when it still exists", () => {
  const p = provider({ quotas: [quota({ id: "five_hour", usedPercent: 18 }), quota({ id: "weekly", usedPercent: 30 })] });
  const primary = getPrimaryQuota(p, { openai: "weekly" });
  assert.equal(primary?.id, "weekly");
  assert.equal(primary?.usedPercent, 30);
});

test("getPrimaryQuota falls back to the first quota when unconfigured", () => {
  const p = provider({ quotas: [quota({ id: "five_hour" }), quota({ id: "weekly" })] });
  assert.equal(getPrimaryQuota(p, {})?.id, "five_hour");
});

test("getPrimaryQuota falls back safely when the configured quota id vanished", () => {
  const p = provider({ quotas: [quota({ id: "five_hour" })] });
  assert.equal(getPrimaryQuota(p, { openai: "removed_quota" })?.id, "five_hour");
});

test("availablePercent is the complement of used, clamped to [0, 100]", () => {
  assert.equal(availablePercent(quota({ usedPercent: 21 })), 79);
  assert.equal(availablePercent(quota({ usedPercent: 0 })), 100);
  assert.equal(availablePercent(quota({ usedPercent: 100 })), 0);
  assert.equal(availablePercent(quota({ usedPercent: 140 })), 0); // clamp above 100% used
  assert.equal(availablePercent(quota({ usedPercent: -10 })), 100); // clamp below 0% used
  assert.equal(availablePercent(undefined), undefined);
});

test("usedPercent and formatPercentage respect percentageMode", () => {
  const q = quota({ usedPercent: 21 });
  assert.equal(usedPercent(q), 21);
  assert.equal(formatPercentage(q, "used"), "21%");
  assert.equal(formatPercentage(q, "remaining"), "79%");
  assert.equal(formatPercentage(q, "available"), "79%");
  assert.equal(formatPercentage(undefined, "used"), "—");
  assert.equal(formatPercentage(quota({ usedPercent: 5.5 }), "used"), "6%");
  assert.equal(formatPercentage(quota({ usedPercent: 5.5 }), "remaining"), "94%");
});

test("visible() filters to enabled providers and orders them per iaUsage.providers", () => {
  const snapshot: DashboardSnapshot = {
    schemaVersion: 1,
    generatedAt: new Date().toISOString(),
    providers: [
      provider({ id: "anthropic", name: "Claude Code" }),
      provider({ id: "openai", name: "Codex" }),
      provider({ id: "cursor", name: "Cursor", enabled: false }),
    ],
  };
  const ordered = visible(snapshot, ["openai", "anthropic"]);
  assert.deepEqual(ordered.map((p) => p.id), ["openai", "anthropic"]);
});

test("shouldWarnBackground fires only when every visible provider is stale", () => {
  assert.equal(shouldWarnBackground([provider({ stale: true }), provider({ stale: false })]), false);
  assert.equal(shouldWarnBackground([provider({ stale: true }), provider({ stale: true })]), true);
  assert.equal(shouldWarnBackground([]), false);
});

test("buildStatusBarLabel: minimal/compact/full density", () => {
  const p = provider();
  assert.equal(buildStatusBarLabel(p, config({ display: "minimal" })), "$(ia-openai) 21%");
  assert.equal(buildStatusBarLabel(p, config({ display: "compact" })), "$(ia-openai) CDX 21%");
  assert.equal(buildStatusBarLabel(p, config({ display: "full" })), "$(ia-openai) Codex / ChatGPT 21%");
});

test("buildStatusBarLabel: icons can be hidden", () => {
  assert.equal(buildStatusBarLabel(provider(), config({ showProviderIcons: false })), "CDX 21%");
});

test("buildStatusBarLabel: remaining mode appends the localized suffix", () => {
  setLang("en");
  assert.equal(buildStatusBarLabel(provider(), config({ percentageMode: "remaining" })), "$(ia-openai) CDX 79% free");
  setLang("es");
  assert.equal(buildStatusBarLabel(provider(), config({ percentageMode: "remaining" })), "$(ia-openai) CDX 79% libre");
  setLang("en");
});

test("buildStatusBarLabel: reset only appears when enabled and the quota has one", () => {
  const p = provider({ quotas: [quota({ resetInSeconds: 39 * 60 })] });
  assert.equal(buildStatusBarLabel(p, config({ showResetInStatusBar: false })), "$(ia-openai) CDX 21%");
  assert.equal(buildStatusBarLabel(p, config({ showResetInStatusBar: true })), "$(ia-openai) CDX 21% (39m)");
});

test("buildStatusBarLabel: stale marker respects showStaleIndicator", () => {
  const stale = provider({ stale: true });
  assert.equal(buildStatusBarLabel(stale, config({ showStaleIndicator: true })), "$(ia-openai) CDX 21%*");
  assert.equal(buildStatusBarLabel(stale, config({ showStaleIndicator: false })), "$(ia-openai) CDX 21%");
});

test("buildStatusBarLabel: honors a custom primary metric", () => {
  const p = provider({ quotas: [quota({ id: "five_hour", usedPercent: 18 }), quota({ id: "weekly", usedPercent: 30 })] });
  assert.equal(buildStatusBarLabel(p, config({ primaryMetric: { openai: "weekly" } })), "$(ia-openai) CDX 30%");
});

test("shared recommendation names the weekly critical quota", () => {
  setLang("es");
  const snapshot: DashboardSnapshot = {
    schemaVersion: 1,
    generatedAt: new Date().toISOString(),
    providers: [provider({ id: "anthropic", name: "Claude Code" }), provider()],
    recommendation: {
      severity: "critical",
      action: "switch",
      fromId: "anthropic",
      toId: "openai",
      toName: "Codex",
      reason: "quota_near_exhaustion",
      confidence: 0.9,
      limitingQuota: {
        id: "weekly", label: "Semanal", windowType: "weekly", usedPercent: 99, availablePercent: 1,
        resetAt: null, resetInSeconds: 126_000, expectedUsedPercent: 79, deltaPercent: 20,
        estimatedExhaustedAt: null, exhaustsBeforeResetSeconds: 122_400,
      },
    },
  };
  const copy = recommendationCopy(snapshot);
  assert.equal(copy?.title, "Claude casi agotado");
  assert.match(copy?.meta ?? "", /Semanal · 99% usado · reinicia en 1 d 11 h/);
  assert.equal(recommendationBackground(snapshot.recommendation), "error");
  setLang("en");
});
