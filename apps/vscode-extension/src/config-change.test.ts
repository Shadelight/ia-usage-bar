import assert from "node:assert/strict";
import { test } from "node:test";
import { isRenderOnlyChange } from "./config-change";

function affects(changed: string[]) {
  return (section: string) => changed.includes(section);
}

test("cosmetic settings do not require a CLI restart", () => {
  assert.equal(isRenderOnlyChange(affects(["iaUsage", "iaUsage.display"])), true);
  assert.equal(isRenderOnlyChange(affects(["iaUsage", "iaUsage.percentageMode"])), true);
  assert.equal(isRenderOnlyChange(affects(["iaUsage", "iaUsage.primaryMetric"])), true);
});

test("cliPath and remotePollSeconds still require a restart", () => {
  assert.equal(isRenderOnlyChange(affects(["iaUsage", "iaUsage.cliPath"])), false);
  assert.equal(isRenderOnlyChange(affects(["iaUsage", "iaUsage.remotePollSeconds"])), false);
});

test("a mixed change (restart key + cosmetic key) still restarts", () => {
  assert.equal(isRenderOnlyChange(affects(["iaUsage", "iaUsage.cliPath", "iaUsage.display"])), false);
});

test("unrelated configuration sections are ignored", () => {
  assert.equal(isRenderOnlyChange(affects(["editor.fontSize"])), false);
});
