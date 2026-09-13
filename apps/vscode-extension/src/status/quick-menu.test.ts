import assert from "node:assert/strict";
import { beforeEach, test } from "node:test";
import * as vscodeMock from "../../test/vscode-mock";
import type { DashboardSnapshot, Provider, UsageQuota } from "../types";
import { pickProviders, reorderProviders, showMenu, showProviderDetail } from "./quick-menu";

beforeEach(() => vscodeMock.__reset());

function quota(overrides: Partial<UsageQuota> = {}): UsageQuota {
  return { id: "session", label: "Session", usedPercent: 21, resetInSeconds: null, stale: false, ...overrides };
}

function provider(overrides: Partial<Provider> = {}): Provider {
  return { id: "openai", name: "Codex", enabled: true, stale: false, quotas: [quota()], ...overrides };
}

function snapshotOf(providers: Provider[]): DashboardSnapshot {
  return { schemaVersion: 1, generatedAt: new Date().toISOString(), providers };
}

function tick(): Promise<void> {
  return new Promise((resolve) => setImmediate(resolve));
}

test("main menu dispatches by action id, not by matching label text", async () => {
  let refreshed = false;
  vscodeMock.__queuePick({ action: "refresh" });
  showMenu(snapshotOf([provider()]), () => { refreshed = true; }, () => {}, () => {});
  await tick();
  assert.equal(refreshed, true);
});

test("'full settings' opens the iaUsage settings page", async () => {
  vscodeMock.__queuePick({ action: "full-settings" });
  showMenu(snapshotOf([provider()]), () => {}, () => {}, () => {});
  await tick();
  const executed = vscodeMock.__executedCommands();
  assert.ok(executed.some((c) => c.id === "workbench.action.openSettings" && c.args[0] === "iaUsage"));
});

test("clicking a provider row opens its detail instead of doing nothing", async () => {
  const p = provider({ id: "openai" });
  vscodeMock.__setConfig("iaUsage", { providers: ["openai"] });
  // First pick selects the provider row; the second answers the detail QuickPick it opens.
  vscodeMock.__queuePick({ action: "provider-detail", providerId: "openai" }, { action: "toggle-visible" });
  showMenu(snapshotOf([p]), () => {}, () => {}, () => {});
  await tick();
  await tick();
  const call = vscodeMock.__updateCalls().find((c) => c.key === "providers");
  assert.deepEqual(call?.value, []); // was visible, detail's toggle removed it
});

test("provider detail 'toggle visible' adds a hidden provider back", async () => {
  vscodeMock.__setConfig("iaUsage", { providers: [] });
  vscodeMock.__queuePick({ action: "toggle-visible" });
  await showProviderDetail(provider({ id: "cursor" }));
  const call = vscodeMock.__updateCalls().find((c) => c.key === "providers");
  assert.deepEqual(call?.value, ["cursor"]);
});

test("pickProviders writes the selection to Global config in snapshot order", async () => {
  vscodeMock.__setConfig("iaUsage", { providers: ["anthropic", "openai", "cursor"] });
  const snapshot = snapshotOf([
    provider({ id: "anthropic", name: "Claude Code" }),
    provider({ id: "openai", name: "Codex" }),
    provider({ id: "cursor", name: "Cursor" }),
  ]);
  vscodeMock.__queuePick([{ providerId: "cursor" }, { providerId: "anthropic" }]);
  await pickProviders(snapshot);
  const call = vscodeMock.__updateCalls().find((c) => c.key === "providers");
  assert.equal(call?.target, vscodeMock.ConfigurationTarget.Global);
  assert.deepEqual(call?.value, ["anthropic", "cursor"]);
});

test("pickProviders with no candidates shows an actionable message instead of a silent no-op", async () => {
  vscodeMock.__setConfig("iaUsage", { providers: [] });
  await pickProviders(snapshotOf([]));
  assert.equal(vscodeMock.__infoMessages().length, 1);
  assert.equal(vscodeMock.__updateCalls().length, 0);
});

test("reorderProviders builds the new order via sequential single picks, not drag/drop", async () => {
  vscodeMock.__setConfig("iaUsage", { providers: ["anthropic", "openai", "cursor"] });
  const snapshot = snapshotOf([
    provider({ id: "anthropic", name: "Claude Code" }),
    provider({ id: "openai", name: "Codex" }),
    provider({ id: "cursor", name: "Cursor" }),
  ]);
  vscodeMock.__queuePick({ providerId: "cursor" }, { providerId: "anthropic" });
  await reorderProviders(snapshot);
  const call = vscodeMock.__updateCalls().find((c) => c.key === "providers");
  assert.deepEqual(call?.value, ["cursor", "anthropic", "openai"]);
});

test("cancelling mid-reorder leaves the existing order untouched", async () => {
  vscodeMock.__setConfig("iaUsage", { providers: ["anthropic", "openai", "cursor"] });
  const snapshot = snapshotOf([
    provider({ id: "anthropic" }),
    provider({ id: "openai" }),
    provider({ id: "cursor" }),
  ]);
  vscodeMock.__queuePick(undefined); // user hits Escape on the first pick
  await reorderProviders(snapshot);
  assert.equal(vscodeMock.__updateCalls().length, 0);
});
