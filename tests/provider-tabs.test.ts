import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

import type { ProviderSnapshot, VendorInfo } from "../src/api.ts";
import { providerShort, tabHtml } from "../src/provider-tabs.ts";

function vendor(partial: Partial<VendorInfo> & { id: string; name: string }): VendorInfo {
  return {
    short: "",
    authKind: "oauth",
    envKey: null,
    hint: "",
    needsKey: false,
    enabled: true,
    detected: true,
    hasCredential: true,
    links: { usageUrl: null, billingUrl: null, statusUrl: null },
    strategies: ["oauth"],
    sourcePreference: null,
    ...partial,
  };
}

function snapshot(partial: Partial<ProviderSnapshot> & { id: string }): ProviderSnapshot {
  return {
    name: partial.id,
    short: "",
    plan: "",
    status: "connected",
    statusReason: null,
    stale: false,
    error: null,
    hint: null,
    updatedAt: "2026-09-13T10:00:00Z",
    quotas: [],
    credits: null,
    productBreakdown: [],
    cost: null,
    lines: [],
    primaryUtilization: null,
    ...partial,
  } as ProviderSnapshot;
}

const visual = { icon: "icon.svg", accent: "#d97757" };
const logo = `<span class="provider-logo-wrap"></span>`;

test("active tab carries real tab semantics plus inline plan badge", () => {
  const html = tabHtml({
    vendor: vendor({ id: "anthropic", name: "Claude Code", short: "CLD" }),
    snapshot: snapshot({ id: "anthropic", plan: "Pro" }),
    active: true,
    loading: false,
    visual,
    logoHtml: logo,
  });
  assert.match(html, /role="tab"/);
  assert.match(html, /aria-selected="true"/);
  assert.match(html, /tabindex="0"/);
  assert.match(html, /aria-label="Claude Code, plan Pro"/);
  assert.match(html, /title="Claude Code · Pro"/);
  assert.match(html, /<span class="label-full">Claude Code<\/span>/);
  assert.match(html, /<span class="label-short">CLD<\/span>/);
  assert.match(html, /<span class="tab-plan">Pro<\/span>/);
  assert.match(html, /status-connected/);
});

test("inactive tabs expose no badge and stay out of tab order", () => {
  const html = tabHtml({
    vendor: vendor({ id: "openai", name: "Codex / ChatGPT", short: "CDX" }),
    snapshot: snapshot({ id: "openai", plan: "Plus" }),
    active: false,
    loading: false,
    visual,
    logoHtml: logo,
  });
  assert.match(html, /aria-selected="false"/);
  assert.match(html, /tabindex="-1"/);
  assert.match(html, /aria-label="Codex \/ ChatGPT, plan Plus"/);
  assert.doesNotMatch(html, /tab-plan/);
  assert.doesNotMatch(html, / active"/);
});

test("loading tabs never invent a plan badge", () => {
  const html = tabHtml({
    vendor: vendor({ id: "cursor", name: "Cursor", short: "CUR" }),
    snapshot: undefined,
    active: true,
    loading: true,
    visual,
    logoHtml: logo,
  });
  assert.match(html, /status-loading/);
  assert.match(html, /aria-label="Cursor"/);
  assert.doesNotMatch(html, /tab-plan/);
});

test("every known backend vendor resolves its explicit unique short", () => {
  const known: Array<[string, string]> = [
    ["anthropic", "CLD"], ["anthropic_api", "ANT"], ["openai", "CDX"],
    ["openai_admin", "OAI"], ["copilot", "COP"], ["zai", "ZAI"],
    ["openrouter", "OR"], ["deepseek", "DSK"], ["kimi", "KMI"],
    ["kilo", "KLO"], ["novita", "NOV"], ["moonshot", "MSH"],
    ["grok", "GRK"], ["supergrok", "SGK"], ["antigravity", "AGY"],
    ["cursor", "CUR"], ["minimax", "MMX"], ["kiro", "KRO"],
    ["nous", "NUS"], ["opencode_go", "OCG"], ["commandcode", "CMD"],
    ["groq", "GRQ"], ["windsurf", "WND"],
  ];
  const seen = new Set<string>();
  for (const [id, short] of known) {
    assert.equal(providerShort(vendor({ id, name: id, short })), short);
    assert.ok(!seen.has(short), `duplicate short ${short}`);
    seen.add(short);
  }
});

test("short fallback derives from the stable id, never name initials", () => {
  assert.equal(providerShort(vendor({ id: "opencode_go", name: "OpenCode Go", short: "" })), "OPEN");
  assert.equal(providerShort(vendor({ id: "some-future_thing!", name: "Claude Clone", short: "  " })), "SOME");
  // Name initials would collide here ("CC"); the id fallback must not.
  assert.notEqual(
    providerShort(vendor({ id: "claude_code", name: "Claude Clone", short: "" })),
    providerShort(vendor({ id: "cursor_cloud", name: "Claude Companion", short: "" })),
  );
});

test("no provider heading survives below the tab rail", () => {
  const dash = readFileSync(new URL("../src/views/dash.ts", import.meta.url), "utf8");
  assert.doesNotMatch(dash, /provider-heading/);
  // The only name+logo repetition left must live in the compact card, where
  // icon-only tabs carry no labels at all.
  const compactIdx = dash.indexOf("compactDetailHtml");
  const providerIdx = dash.indexOf("compact-provider");
  assert.ok(compactIdx !== -1 && providerIdx > compactIdx);
});

test("stylesheet carries the responsive tab contract", () => {
  const css = readFileSync(new URL("../src/styles.css", import.meta.url), "utf8");
  assert.match(css, /\.tab \.label-short/);
  assert.match(css, /@media \(max-width: 499px\)/);
  assert.match(css, /@media \(min-width: 500px\) and \(max-width: 750px\)/);
  assert.match(css, /\.tab:not\(\.active\) \.label-short/);
  assert.match(css, /\.tab-plan/);
});
