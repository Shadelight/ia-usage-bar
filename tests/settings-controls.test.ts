import assert from "node:assert/strict";
import test from "node:test";
import { readFileSync } from "node:fs";

// Regression tests for the v0.2.0 settings breakage (issue: checkboxes
// never stuck, selects died on open, rows stuck expanded, silent failures):
// - every click fell through `closest()` to `<html data-theme>` and hit the
//   theme branch, rebuilding settings mid-gesture;
// - the expand toggle searched `[data-provider]` while rows carry
//   `data-provider-item`, so rows never collapsed;
// - enable/save/detect failures were silent (no toast, no revert).

const main = readFileSync(new URL("../src/main.ts", import.meta.url), "utf8");

test("theme branch ignores the documentElement fallthrough", () => {
  assert.match(
    main,
    /btn !== document\.documentElement && btn\.dataset\.theme/,
    "clicks on bare controls must not enter the theme branch via <html data-theme>",
  );
});

test("expand toggle queries the real row attribute", () => {
  assert.ok(
    main.includes('closest("[data-provider-item]")'),
    "rows render data-provider-item, so the toggle must query it",
  );
  assert.ok(
    !main.includes('closest("[data-provider]")'),
    "[data-provider] never matches and leaves rows stuck open",
  );
});

test("failed enable reverts the checkbox and reports", () => {
  const changeHandler = main.slice(main.indexOf("dataset.enable"));
  assert.match(changeHandler, /showCommandError\(t\("commandFailed"\)\)/);
  // Model reverts in all variants.
  assert.match(changeHandler, /vendor\.enabled = !enabled/);
  // Form reverts either by direct node write or by collapsing + re-render.
  assert.ok(
    changeHandler.includes("input.checked = !enabled") ||
      changeHandler.includes("setExpandedProvider(null)"),
    "must revert the visible checkbox on backend failure",
  );
});

test("failed save/detect report instead of staying silent", () => {
  const saveKey = main.slice(main.indexOf("dataset.savekey"));
  assert.match(saveKey, /showCommandError\(t\("commandFailed"\)\)/);
  const detect = main.slice(main.indexOf('hasAttribute("data-detect")'));
  assert.match(detect, /showCommandError\(t\("commandFailed"\)\)/);
});
