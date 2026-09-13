import assert from "node:assert/strict";
import { test } from "node:test";
import { providerVisual } from "./provider-visuals";

test("known providers map to their brand icon and short code", () => {
  assert.deepEqual(providerVisual("anthropic", "Claude Code"), { icon: "ia-anthropic", short: "CLD" });
  assert.deepEqual(providerVisual("openai", "Codex"), { icon: "ia-openai", short: "CDX" });
  assert.deepEqual(providerVisual("antigravity", "Antigravity"), { icon: "ia-antigravity", short: "AGY" });
  assert.deepEqual(providerVisual("opencode_go", "OpenCode Go"), { icon: "ia-opencode_go", short: "OCG" });
});

test("an unknown provider never breaks: generic icon + derived short code", () => {
  const visual = providerVisual("some-new-vendor", "Some New Vendor");
  assert.equal(visual.icon, "circle-filled");
  assert.equal(visual.short, "SOM");
});

test("a name with no alphanumerics still produces a 3-letter fallback code", () => {
  const visual = providerVisual("mystery", "!!!");
  assert.equal(visual.icon, "circle-filled");
  assert.equal(visual.short, "AI");
});
