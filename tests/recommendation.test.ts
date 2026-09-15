import assert from "node:assert/strict";
import test from "node:test";

import type { Dashboard, RecommendCandidate } from "../src/api.ts";
import { setLang } from "../src/i18n.ts";
import { durLabelSecs, recCopy, recPrefix, recWhyTitle } from "../src/recommend.ts";

function cand(partial: Partial<RecommendCandidate> & { id: string; name: string }): RecommendCandidate {
  return {
    score: 50,
    shortHeadroom: null,
    longHeadroom: null,
    displayLeft: null,
    sustainable: null,
    exhaustInSecs: null,
    resetInSecs: null,
    hasData: true,
    isReserve: false,
    ...partial,
  };
}

function dash(partial: Partial<Dashboard>): Dashboard {
  return {
    providers: [],
    catalog: [
      { id: "anthropic", name: "Claude Code", short: "CLD", authKind: "oauth", envKey: null, hint: "", needsKey: false, enabled: true, detected: true, hasCredential: true, links: { usageUrl: null, billingUrl: null, statusUrl: null }, strategies: ["oauth"], sourcePreference: null },
      { id: "openai", name: "Codex", short: "CDX", authKind: "oauth", envKey: null, hint: "", needsKey: false, enabled: true, detected: true, hasCredential: true, links: { usageUrl: null, billingUrl: null, statusUrl: null }, strategies: ["oauth"], sourcePreference: null },
      { id: "opencode", name: "OpenCode Go", short: "OCG", authKind: "apikey", envKey: null, hint: "", needsKey: true, enabled: true, detected: true, hasCredential: true, links: { usageUrl: null, billingUrl: null, statusUrl: null }, strategies: ["api"], sourcePreference: null },
      { id: "cursor", name: "Cursor", short: "CUR", authKind: "oauth", envKey: null, hint: "", needsKey: false, enabled: true, detected: true, hasCredential: true, links: { usageUrl: null, billingUrl: null, statusUrl: null }, strategies: ["oauth"], sourcePreference: null },
    ],
    refreshMinutes: 5,
    refreshAdaptive: true,
    primary: "anthropic",
    notifications: true,
    notifyThresholds: [75, 90, 95],
    autostart: false,
    alwaysOnTop: false,
    compactMode: false,
    appBootstrapping: false,
    refreshing: false,
    loadingProviders: [],
    nextUpdateInSecs: 60,
    spendMonthUsd: 0,
    spend: [],
    recommendId: null,
    recommendName: null,
    recommendLeft: null,
    recommendAction: "",
    recommendReason: "",
    recommendSeverity: "",
    recommendLimitingQuota: null,
    recommendConfidence: 0,
    recommendFrom: null,
    recommendScores: [],
    ...partial,
  };
}

test("stay/sustainable keeps Claude instead of chasing 100%", () => {
  setLang("es");
  const d = dash({
    recommendId: "anthropic",
    recommendName: "Claude Code",
    recommendLeft: 35,
    recommendAction: "stay",
    recommendReason: "sustainable",
    recommendConfidence: 0.85,
    recommendFrom: "anthropic",
    recommendScores: [
      cand({ id: "anthropic", name: "Claude Code", score: 74, shortHeadroom: 35, displayLeft: 35, sustainable: true, resetInSecs: 9000 }),
      cand({ id: "opencode", name: "OpenCode Go", score: 30, hasData: false, isReserve: true }),
    ],
  });
  const rec = recCopy(d, "anthropic");
  assert.ok(rec);
  assert.equal(rec.selectId, "anthropic");
  assert.match(rec.text, /Sigue con Claude Code/);
  assert.equal(recPrefix(d), "✓ ");
});

test("switch/exhausts names the exhaustion gap, not just headroom", () => {
  setLang("es");
  const d = dash({
    recommendId: "openai",
    recommendName: "Codex",
    recommendLeft: 80,
    recommendAction: "switch",
    recommendReason: "exhausts_before_reset",
    recommendConfidence: 0.9,
    recommendFrom: "anthropic",
    recommendScores: [
      cand({ id: "openai", name: "Codex", score: 81, shortHeadroom: 80, displayLeft: 80, sustainable: true, resetInSecs: 18000 }),
      cand({ id: "anthropic", name: "Claude Code", score: 48, shortHeadroom: 12, displayLeft: 12, sustainable: false, exhaustInSecs: 6300, resetInSecs: 14400 }),
    ],
  });
  const rec = recCopy(d, "anthropic");
  assert.ok(rec);
  assert.equal(rec.selectId, "openai");
  assert.match(rec.text, /Conviene cambiar a Codex/);
  assert.match(rec.text, /antes del reset/);
  assert.equal(recPrefix(d), "↗ ");
});

test("switch/critical uses the urgent copy", () => {
  setLang("es");
  const d = dash({
    recommendId: "openai",
    recommendName: "Codex",
    recommendLeft: 80,
    recommendAction: "switch",
    recommendReason: "critical_short",
    recommendConfidence: 0.9,
    recommendFrom: "anthropic",
    recommendScores: [
      cand({ id: "openai", name: "Codex", score: 81, displayLeft: 80, sustainable: true }),
      cand({ id: "anthropic", name: "Claude Code", score: 30, displayLeft: 5, sustainable: false, exhaustInSecs: 2520, resetInSecs: 14100 }),
    ],
  });
  const rec = recCopy(d, "anthropic");
  assert.ok(rec);
  assert.match(rec.text, /Cambia a Codex/);
  assert.equal(recPrefix(d), "⛔ ");
});

test("stay/at_risk warns without ordering a switch", () => {
  setLang("es");
  const d = dash({
    recommendId: "anthropic",
    recommendName: "Claude Code",
    recommendLeft: 18,
    recommendAction: "stay",
    recommendReason: "at_risk",
    recommendConfidence: 0.7,
    recommendFrom: "anthropic",
    recommendScores: [
      cand({ id: "anthropic", name: "Claude Code", score: 60, shortHeadroom: 18, displayLeft: 18, sustainable: true }),
      cand({ id: "openai", name: "Codex", score: 68, shortHeadroom: 55, displayLeft: 55, sustainable: true }),
    ],
  });
  const rec = recCopy(d, "anthropic");
  assert.ok(rec);
  assert.match(rec.text, /va justo/);
  assert.match(rec.text, /Codex/);
  assert.doesNotMatch(rec.text, /Conviene cambiar/);
});

test("reserve names the 100%-empty provider without recommending it", () => {
  setLang("es");
  const d = dash({
    recommendId: "anthropic",
    recommendName: "Claude Code",
    recommendLeft: 35,
    recommendAction: "stay",
    recommendReason: "reserve_no_history",
    recommendConfidence: 0.6,
    recommendFrom: "anthropic",
    recommendScores: [
      cand({ id: "anthropic", name: "Claude Code", score: 70, displayLeft: 35, sustainable: true }),
      cand({ id: "opencode", name: "OpenCode Go", score: 30, hasData: false, isReserve: true }),
    ],
  });
  const rec = recCopy(d, "anthropic");
  assert.ok(rec);
  assert.match(rec.text, /OpenCode Go/);
  assert.match(rec.text, /sin historial/);
  assert.doesNotMatch(rec.text, /Cambia a OpenCode/);
});

test("balanced and insufficient_data behave", () => {
  setLang("es");
  const balanced = dash({
    recommendId: "anthropic",
    recommendName: "Claude Code",
    recommendLeft: 40,
    recommendAction: "balanced",
    recommendReason: "balanced",
    recommendConfidence: 0.7,
    recommendScores: [],
  });
  assert.match(recCopy(balanced, "anthropic")!.text, /equilibradas/);
  assert.equal(recPrefix(balanced), "= ");

  const empty = dash({
    recommendId: null,
    recommendName: null,
    recommendLeft: null,
    recommendAction: "insufficient_data",
    recommendReason: "insufficient_data",
    recommendConfidence: 0.3,
    recommendScores: [],
  });
  assert.equal(recCopy(empty, "anthropic"), null);
});

test("legacy payloads without the engine keep the old copy", () => {
  setLang("es");
  const d = dash({ recommendId: "openai", recommendName: "Codex", recommendLeft: 74 });
  assert.equal(recCopy(d, "openai")!.text, "Codex tiene el mayor margen (74%)");
  assert.equal(recCopy(d, "anthropic")!.text, "Cambia a Codex — 74% de margen");
});

test("why-title explains every candidate plus confidence", () => {
  setLang("es");
  const d = dash({
    recommendId: "anthropic",
    recommendName: "Claude Code",
    recommendLeft: 35,
    recommendAction: "stay",
    recommendReason: "sustainable",
    recommendConfidence: 0.85,
    recommendScores: [
      cand({ id: "anthropic", name: "Claude Code", score: 74, displayLeft: 35, sustainable: true, resetInSecs: 9000 }),
      cand({ id: "openai", name: "Codex", score: 60, displayLeft: 59, sustainable: false, resetInSecs: 19000 }),
      cand({ id: "opencode", name: "OpenCode Go", score: 30, hasData: false, isReserve: true }),
    ],
  });
  const title = recWhyTitle(d);
  assert.match(title, /Claude Code: 35% · llega al reset/);
  assert.match(title, /Codex: 59% · se agotaría antes del reset/);
  assert.match(title, /OpenCode Go: — · sin historial suficiente/);
  assert.match(title, /Confianza alta/);
});

test("recommendation i18n keys exist in both languages", () => {
  for (const lang of ["es", "en"] as const) {
    setLang(lang);
    const d = dash({
      recommendId: "openai",
      recommendName: "Codex",
      recommendLeft: 80,
      recommendAction: "switch",
      recommendReason: "exhausts_before_reset",
      recommendConfidence: 0.9,
      recommendFrom: "anthropic",
      recommendScores: [
        cand({ id: "openai", name: "Codex", score: 81, displayLeft: 80, sustainable: true }),
        cand({ id: "anthropic", name: "Claude Code", score: 48, displayLeft: 12, sustainable: false, exhaustInSecs: 6300, resetInSecs: 14400 }),
      ],
    });
    const text = recCopy(d, "anthropic")!.text;
    assert.ok(!text.includes("{name}") && !text.includes("{from}") && !text.includes("{dur}"), `${lang}: ${text}`);
  }
  setLang("es");
});

test("durations render compactly", () => {
  assert.equal(durLabelSecs(45 * 60), "45 min");
  assert.equal(durLabelSecs(6300), "1h 45m");
  assert.equal(durLabelSecs(9000), "2h 30m");
  assert.equal(durLabelSecs(3 * 86_400 + 3600), "3d 1h");
});

test("weekly critical quota drives the banner instead of the session forecast", () => {
  setLang("es");
  const d = dash({
    recommendId: "openai",
    recommendName: "Codex",
    recommendLeft: 60,
    recommendAction: "switch",
    recommendReason: "quota_near_exhaustion",
    recommendSeverity: "critical",
    recommendConfidence: 0.9,
    recommendFrom: "anthropic",
    recommendLimitingQuota: {
      id: "weekly",
      label: "Semanal",
      windowType: "weekly",
      usedPercent: 99,
      availablePercent: 1,
      resetAt: "2026-09-16T00:00:00Z",
      resetInSeconds: 126_000,
      expectedUsedPercent: 79,
      deltaPercent: 20,
      estimatedExhaustedAt: "2026-09-14T14:00:00Z",
      exhaustsBeforeResetSeconds: 122_400,
    },
  });
  const rec = recCopy(d, "anthropic");
  assert.ok(rec);
  assert.equal(rec.severity, "critical");
  assert.equal(rec.title, "Claude casi agotado");
  assert.match(rec.meta, /Semanal · 99% usado · reinicia en 1d 11h/);
  assert.ok(rec.detailLines.some((line) => /Codex es la mejor alternativa/.test(line)));
  assert.doesNotMatch(rec.text, /sesión|53 min/i);
});

test("partial_limited generates warning banner with some models exhausted", () => {
  setLang("es");
  const d = dash({
    recommendId: "cursor",
    recommendName: "Cursor",
    recommendLeft: 62,
    recommendAction: "stay",
    recommendReason: "partial_limited",
    recommendSeverity: "warning",
    recommendConfidence: 0.9,
    recommendFrom: "cursor",
    recommendLimitingQuota: {
      id: "other_models",
      label: "Other Models",
      windowType: "monthly",
      usedPercent: 100,
      availablePercent: 0,
      resetAt: "2026-09-23T00:00:00Z",
      resetInSeconds: 600_000,
    },
    recommendScores: [
      cand({ id: "cursor", name: "Cursor", score: 65, displayLeft: 62, sustainable: true }),
    ],
  });
  const rec = recCopy(d, "cursor");
  assert.ok(rec);
  assert.equal(rec.severity, "warning");
  assert.equal(rec.title, "Cursor tiene algunos modelos agotados");
  assert.match(rec.meta, /Other Models ha alcanzado su límite/);
});

test("switch copy is contextual when already viewing the destination", () => {
  setLang("es");
  const d = dash({
    recommendId: "antigravity",
    recommendName: "Antigravity",
    recommendLeft: 85,
    recommendAction: "switch",
    recommendReason: "critical_short",
    recommendSeverity: "critical",
    recommendFrom: "anthropic",
    recommendScores: [
      cand({ id: "anthropic", name: "Claude Code", exhaustInSecs: 60 }),
      cand({ id: "antigravity", name: "Antigravity", displayLeft: 85 }),
    ],
  });
  const here = recCopy(d, "antigravity");
  assert.ok(here);
  assert.match(here.title, /Ya estás en Antigravity/);
  assert.doesNotMatch(here.title, /Cambia a/);
  const from = recCopy(d, "anthropic");
  assert.ok(from);
  assert.match(from.title, /Claude/);
});
