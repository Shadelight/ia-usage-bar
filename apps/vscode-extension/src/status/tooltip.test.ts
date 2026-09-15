import assert from "node:assert/strict";
import { beforeEach, test } from "node:test";
import * as vscodeMock from "../../test/vscode-mock";
import { setLang } from "../i18n";
import type { DashboardSnapshot, Provider, UsageQuota } from "../types";
import { tooltip } from "./tooltip";

beforeEach(() => {
  vscodeMock.__reset();
  setLang("en");
});

function quota(overrides: Partial<UsageQuota> = {}): UsageQuota {
  return { id: "session", label: "Session", usedPercent: 21, resetInSeconds: null, stale: false, ...overrides };
}

function provider(overrides: Partial<Provider> = {}): Provider {
  return { id: "openai", name: "Codex / ChatGPT", enabled: true, stale: false, updatedAt: new Date().toISOString(), quotas: [quota()], ...overrides };
}

function snapshot(providers: Provider[]): DashboardSnapshot {
  return { schemaVersion: 1, generatedAt: new Date().toISOString(), providers };
}

test("shows used and remaining together even though the bar shows only one", () => {
  const md = tooltip(snapshot([provider()]), [provider()], {});
  assert.match(md.value, /21% used · 79% remaining/);
});

test("reset appears on its own line via 'Resets in'", () => {
  const p = provider({ quotas: [quota({ resetInSeconds: 39 * 60 })] });
  const md = tooltip(snapshot([p]), [p], {});
  assert.match(md.value, /Resets in 39 min/);
});

test("stale providers show the stale wording, not a 0%", () => {
  const p = provider({ stale: true, quotas: [quota({ usedPercent: 21 })] });
  const md = tooltip(snapshot([p]), [p], {});
  assert.match(md.value, /Stale data/);
  assert.match(md.value, /21% used/);
});

test("Codex tooltip retains supplemental credits and reset credits with independent freshness", () => {
  const p = provider({ credits: { remaining: 298, resetsAvailable: 1, stale: true, resetsStale: false } });
  const md = tooltip(snapshot([p]), [p], {});
  assert.match(md.value, /298/);
  assert.match(md.value, /1/);
  assert.match(md.value, /Stale data/);
});

test("caps at 5 providers and collapses the rest into a '+N more' line", () => {
  const providers = Array.from({ length: 7 }, (_, i) => provider({ id: `p${i}`, name: `Provider ${i}` }));
  const md = tooltip(snapshot(providers), providers, {});
  const shownNames = providers.slice(0, 5).map((p) => p.name);
  for (const name of shownNames) assert.ok(md.value.includes(name), `expected ${name} to be rendered`);
  assert.ok(!md.value.includes("Provider 5"));
  assert.match(md.value, /\+2 more configured providers/);
});

test("showAllMetrics=false still shows always quotas at 0%", () => {
  vscodeMock.__setConfig("iaUsage", { showAllMetrics: false });
  const p = provider({ quotas: [quota({ id: "a", label: "Five hour", usedPercent: 21 }), quota({ id: "b", label: "Weekly", usedPercent: 0 })] });
  const md = tooltip(snapshot([p]), [p], {});
  assert.ok(md.value.includes("Five hour"));
  assert.ok(md.value.includes("Weekly"));
});

test("showAllMetrics=false hides details quotas", () => {
  vscodeMock.__setConfig("iaUsage", { showAllMetrics: false });
  const p = provider({
    quotas: [
      quota({ id: "a", label: "Five hour", usedPercent: 21 }),
      quota({ id: "total", label: "Total", usedPercent: 40, visible: "details" }),
    ],
  });
  const md = tooltip(snapshot([p]), [p], {});
  assert.ok(md.value.includes("Five hour"));
  assert.ok(!md.value.includes("Total"));
});

test("showAllMetrics=true shows every quota, including 0%", () => {
  vscodeMock.__setConfig("iaUsage", { showAllMetrics: true });
  const p = provider({ quotas: [quota({ id: "a", label: "Five hour", usedPercent: 21 }), quota({ id: "b", label: "Weekly", usedPercent: 0 })] });
  const md = tooltip(snapshot([p]), [p], {});
  assert.ok(md.value.includes("Five hour"));
  assert.ok(md.value.includes("Weekly"));
});

test("Antigravity tooltip groups windows and keeps a 0% family visible", () => {
  const p = provider({
    id: "antigravity",
    name: "Antigravity",
    quotas: [
      quota({ id: "gemini_models_weekly", label: "weekly", windowType: "weekly", groupId: "gemini_models", groupLabel: "Gemini Models", models: ["Gemini Flash"], usedPercent: 5.48 }),
      quota({ id: "gemini_models_5h", label: "5h", windowType: "5h", groupId: "gemini_models", groupLabel: "Gemini Models", usedPercent: 14.88 }),
      quota({ id: "claude_gpt_models_weekly", label: "weekly", windowType: "weekly", groupId: "claude_gpt_models", groupLabel: "Claude + GPT", usedPercent: 0 }),
      quota({ id: "claude_gpt_models_5h", label: "5h", windowType: "5h", groupId: "claude_gpt_models", groupLabel: "Claude + GPT", usedPercent: 0 }),
    ],
  });
  const md = tooltip(snapshot([p]), [p], {});
  assert.ok(md.value.includes("Gemini Models"));
  assert.match(md.value, /Claude \\\+ GPT/);
  assert.match(md.value, /5% used · 95% remaining/);
  assert.match(md.value, /15% used · 85% remaining/);
  assert.match(md.value, /0% used · 100% remaining/);
});
