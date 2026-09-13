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

test("shows used and available together even though the bar shows only one", () => {
  const md = tooltip(snapshot([provider()]), [provider()], {});
  assert.match(md.value, /21% used · 79% available/);
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

test("caps at 5 providers and collapses the rest into a '+N more' line", () => {
  const providers = Array.from({ length: 7 }, (_, i) => provider({ id: `p${i}`, name: `Provider ${i}` }));
  const md = tooltip(snapshot(providers), providers, {});
  const shownNames = providers.slice(0, 5).map((p) => p.name);
  for (const name of shownNames) assert.ok(md.value.includes(name), `expected ${name} to be rendered`);
  assert.ok(!md.value.includes("Provider 5"));
  assert.match(md.value, /\+2 more configured providers/);
});

test("showAllMetrics=false hides secondary quotas stuck at 0%", () => {
  vscodeMock.__setConfig("iaUsage", { showAllMetrics: false });
  const p = provider({ quotas: [quota({ id: "a", label: "Five hour", usedPercent: 21 }), quota({ id: "b", label: "Weekly", usedPercent: 0 })] });
  const md = tooltip(snapshot([p]), [p], {});
  assert.ok(md.value.includes("Five hour"));
  assert.ok(!md.value.includes("Weekly"));
});

test("showAllMetrics=true shows every quota, including 0%", () => {
  vscodeMock.__setConfig("iaUsage", { showAllMetrics: true });
  const p = provider({ quotas: [quota({ id: "a", label: "Five hour", usedPercent: 21 }), quota({ id: "b", label: "Weekly", usedPercent: 0 })] });
  const md = tooltip(snapshot([p]), [p], {});
  assert.ok(md.value.includes("Five hour"));
  assert.ok(md.value.includes("Weekly"));
});
