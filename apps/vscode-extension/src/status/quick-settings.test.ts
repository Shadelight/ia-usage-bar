import assert from "node:assert/strict";
import { beforeEach, test } from "node:test";
import * as vscodeMock from "../../test/vscode-mock";
import type { Provider, UsageQuota } from "../types";
import { choosePrimaryMetricForProvider, showQuickSettings } from "./quick-settings";

beforeEach(() => vscodeMock.__reset());

function quota(overrides: Partial<UsageQuota> = {}): UsageQuota {
  return { id: "five_hour", label: "5 hours", usedPercent: 21, resetInSeconds: 39 * 60, stale: false, ...overrides };
}

function provider(overrides: Partial<Provider> = {}): Provider {
  return { id: "openai", name: "Codex / ChatGPT", enabled: true, stale: false, quotas: [quota()], ...overrides };
}

function updateFor(key: string) {
  return vscodeMock.__updateCalls().find((c) => c.key === key);
}

test("choosing a primary metric stores the stable quota id, not its (possibly localized) label", async () => {
  const p = provider({ quotas: [quota({ id: "five_hour", label: "5 hours" }), quota({ id: "weekly", label: "Weekly"})] });
  vscodeMock.__queuePick({ quotaId: "weekly" });
  await choosePrimaryMetricForProvider(p);
  const call = updateFor("primaryMetric");
  assert.equal(call?.target, vscodeMock.ConfigurationTarget.Global);
  assert.deepEqual(call?.value, { openai: "weekly" });
});

test("appearance quick settings write display/icons/stale to Global, applied instantly", async () => {
  vscodeMock.__setConfig("iaUsage", { display: "compact", showProviderIcons: true, showStaleIndicator: true });
  vscodeMock.__queuePick(
    { action: "appearance" },
    { value: "full" },
    { value: "off" },
    { value: "off" },
  );
  await showQuickSettings(undefined);
  assert.equal(updateFor("display")?.value, "full");
  assert.equal(updateFor("showProviderIcons")?.value, false);
  assert.equal(updateFor("showStaleIndicator")?.value, false);
  for (const key of ["display", "showProviderIcons", "showStaleIndicator"]) {
    assert.equal(updateFor(key)?.target, vscodeMock.ConfigurationTarget.Global);
  }
});

test("percentage quick setting writes remaining, never the available alias", async () => {
  vscodeMock.__queuePick({ action: "percentage" }, { value: "remaining" });
  await showQuickSettings(undefined);
  assert.equal(updateFor("percentageMode")?.value, "remaining");
});

test("reset quick setting writes showResetInStatusBar", async () => {
  vscodeMock.__queuePick({ action: "reset" }, { value: "on" });
  await showQuickSettings(undefined);
  assert.equal(updateFor("showResetInStatusBar")?.value, true);
});

test("tooltip info quick setting writes showAllMetrics", async () => {
  vscodeMock.__queuePick({ action: "tooltip" }, { value: "all" });
  await showQuickSettings(undefined);
  assert.equal(updateFor("showAllMetrics")?.value, true);
});

test("'open full settings' delegates to the native settings UI instead of a custom webview", async () => {
  vscodeMock.__queuePick({ action: "full-settings" });
  await showQuickSettings(undefined);
  assert.ok(vscodeMock.__executedCommands().some((c) => c.id === "workbench.action.openSettings" && c.args[0] === "iaUsage"));
});
