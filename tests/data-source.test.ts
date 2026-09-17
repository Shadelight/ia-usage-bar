import assert from "node:assert/strict";
import test from "node:test";

import type { ProviderSnapshot, VendorInfo } from "../src/api.ts";
import {
  assertSnapshotMatchesProvider,
  dataSourceLabel,
  detailsForSelectedProvider,
  providerDetails,
} from "../src/data-source.ts";
import { setLang } from "../src/i18n.ts";

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
    name: partial.name ?? partial.id,
    short: "",
    plan: "",
    status: "connected",
    statusReason: null,
    stale: false,
    error: null,
    hint: null,
    updatedAt: "2026-09-16T18:00:00Z",
    quotas: [],
    credits: null,
    productBreakdown: [],
    cost: null,
    lines: [],
    primaryUtilization: null,
    ...partial,
  };
}

const vendors: VendorInfo[] = [
  vendor({ id: "antigravity", name: "Antigravity", authKind: "local", strategies: ["local"] }),
  vendor({ id: "cursor", name: "Cursor", authKind: "local", strategies: ["local"] }),
  vendor({ id: "opencode_go", name: "OpenCode Go", authKind: "apikey", needsKey: true, strategies: ["api"] }),
  vendor({ id: "openai", name: "Codex / ChatGPT", authKind: "oauth", strategies: ["oauth"] }),
];

const antigravity = snapshot({
  id: "antigravity",
  name: "Antigravity",
  plan: "Free",
  activeSource: "local-session",
});
const cursor = snapshot({
  id: "cursor",
  name: "Cursor",
  plan: "Pro",
  activeSource: "local-session",
});
const opencodeGo = snapshot({
  id: "opencode_go",
  name: "OpenCode Go",
  plan: "OpenCode Go",
  activeSource: "api",
});
const openai = snapshot({
  id: "openai",
  name: "Codex / ChatGPT",
  plan: "Plus",
  activeSource: "oauth",
});

test("Cursor local never inherits Antigravity local", () => {
  setLang("es");
  assert.equal(dataSourceLabel(antigravity), "Antigravity local");
  assert.equal(dataSourceLabel(cursor), "Cursor local");
  assert.doesNotMatch(dataSourceLabel(cursor), /Antigravity/);
  assert.doesNotMatch(dataSourceLabel(cursor), /Google API/);
});

test("Codex never inherits OpenCode Go Google API", () => {
  setLang("es");
  assert.equal(dataSourceLabel(opencodeGo), "API de OpenCode Go");
  assert.equal(dataSourceLabel(openai), "Codex / ChatGPT");
  assert.doesNotMatch(dataSourceLabel(openai), /Google API/);
  assert.doesNotMatch(dataSourceLabel(openai), /OpenCode/);
});

test("sequential tab selection keeps source/auth/plan owned by each provider", () => {
  setLang("es");
  const snapshots = [cursor, openai, antigravity, opencodeGo];
  const order = ["cursor", "openai", "antigravity", "opencode_go"] as const;
  const seen: Record<string, ReturnType<typeof detailsForSelectedProvider>> = {};
  for (const id of order) {
    seen[id] = detailsForSelectedProvider(snapshots, vendors, id);
  }
  assert.equal(seen.cursor?.source, "Cursor local");
  assert.equal(seen.cursor?.plan, "Pro");
  assert.equal(seen.cursor?.auth, "Local");
  assert.equal(seen.openai?.source, "Codex / ChatGPT");
  assert.equal(seen.openai?.plan, "Plus");
  assert.equal(seen.openai?.auth, "OAuth");
  assert.equal(seen.antigravity?.source, "Antigravity local");
  assert.equal(seen.opencode_go?.source, "API de OpenCode Go");
  assert.equal(seen.opencode_go?.plan, "OpenCode Go");
  assert.equal(seen.opencode_go?.auth, "API key");
  assert.doesNotMatch(seen.cursor?.source || "", /Antigravity|Google API/);
  assert.doesNotMatch(seen.openai?.source || "", /Google API|OpenCode/);
});

test("partial metadata becomes empty, never the previous provider's values", () => {
  setLang("es");
  const previous = detailsForSelectedProvider([antigravity], vendors, "antigravity");
  assert.equal(previous?.source, "Antigravity local");
  const partial = snapshot({ id: "cursor", name: "Cursor", plan: "", activeSource: null });
  const next = detailsForSelectedProvider([partial], vendors, "cursor");
  assert.equal(next?.source, "");
  assert.equal(next?.plan, "");
  assert.notEqual(next?.source, previous?.source);
});

test("concurrent refresh of every snapshot cannot contaminate another", async () => {
  setLang("es");
  const snapshots = [antigravity, cursor, opencodeGo, openai];
  const ids = snapshots.map((item) => item.id);
  const results = await Promise.all(
    ids.map((id) => Promise.resolve(detailsForSelectedProvider(snapshots, vendors, id))),
  );
  const byId = Object.fromEntries(ids.map((id, index) => [id, results[index]]));
  assert.equal(byId.cursor?.source, "Cursor local");
  assert.equal(byId.antigravity?.source, "Antigravity local");
  assert.equal(byId.openai?.source, "Codex / ChatGPT");
  assert.equal(byId.opencode_go?.source, "API de OpenCode Go");
});

test("rapid tab switches always resolve DETAILS from the selected providerId", () => {
  setLang("es");
  const snapshots = [antigravity, cursor, opencodeGo, openai];
  const tabs = ["antigravity", "cursor", "cursor", "openai", "opencode_go", "cursor"];
  const last = tabs.map((id) => detailsForSelectedProvider(snapshots, vendors, id));
  assert.equal(last[1]?.providerId, "cursor");
  assert.equal(last[2]?.providerId, "cursor");
  assert.equal(last[5]?.source, "Cursor local");
  assert.equal(last[3]?.source, "Codex / ChatGPT");
  assert.equal(last[4]?.source, "API de OpenCode Go");
});

test("invariant rejects painting one snapshot inside another provider", () => {
  assert.throws(
    () => assertSnapshotMatchesProvider(antigravity, "cursor"),
    /antigravity.*cursor/,
  );
  assert.throws(() => providerDetails(antigravity, vendors.find((item) => item.id === "cursor")!));
});

test("Antigravity oauth is the only Google API data source", () => {
  setLang("es");
  const cloud = snapshot({ id: "antigravity", name: "Antigravity", activeSource: "oauth" });
  assert.equal(dataSourceLabel(cloud), "Google API");
  assert.doesNotMatch(dataSourceLabel(openai), /Google API/);
  assert.doesNotMatch(dataSourceLabel(opencodeGo), /Google API/);
});

test("English local labels stay provider-owned", () => {
  setLang("en");
  assert.equal(dataSourceLabel(cursor), "Cursor local");
  assert.equal(dataSourceLabel(opencodeGo), "OpenCode Go API");
  assert.equal(dataSourceLabel(openai), "Codex / ChatGPT");
});
