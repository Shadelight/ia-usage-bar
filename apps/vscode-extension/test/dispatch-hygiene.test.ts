import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { test } from "node:test";

/** Section 15 of the spec is explicit: menu actions must be dispatched via a
 * typed `action`/`providerId` field, never by pattern-matching the
 * (possibly localized) label text. This is a static guard against
 * regressing back to `label.includes(...)` dispatch. */
const FILES = ["src/status/quick-menu.ts", "src/status/quick-settings.ts"];

test("menu dispatch never matches on label text", () => {
  for (const file of FILES) {
    const source = readFileSync(join(__dirname, "..", "..", file), "utf8");
    assert.doesNotMatch(source, /\.label\s*(===|==|\.includes\()/, `${file} must dispatch via action ids, not label matching`);
    assert.doesNotMatch(source, /choice\.label/, `${file} must not branch on choice.label`);
  }
});

test("QuickPick items carry a typed action/providerId, not free-form strings", () => {
  const quickMenu = readFileSync(join(__dirname, "..", "..", "src/status/quick-menu.ts"), "utf8");
  assert.match(quickMenu, /interface UsageQuickPickItem extends vscode\.QuickPickItem/);
  assert.match(quickMenu, /action\?:\s*MenuAction/);
});
