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
const settings = readFileSync(new URL("../src/views/settings.ts", import.meta.url), "utf8");

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

test("void Rust commands use an explicit outcome instead of null as success", () => {
  const api = readFileSync(new URL("../src/api.ts", import.meta.url), "utf8");
  assert.match(api, /type CommandResult<T>/);
  assert.match(api, /return \{ ok: true, value: await invoke<T>\(cmd, args\) \}/);
  assert.doesNotMatch(main, /invokeCmd[^\n]*!== null/);
});

test("dashboard refreshes patch Settings instead of rebuilding its controls", () => {
  assert.match(main, /if \(view === "settings"\) \{[\s\S]*?patchSettings\(dash, settingsCategory\);/);
  assert.match(main, /pendingProviderEnabled/);
  assert.match(main, /pendingSourcePreferences/);
});

test("the Settings patch does not replace native inputs during a refresh", () => {
  const patchBody = settings.slice(settings.indexOf("export function patchSettings"));
  assert.match(patchBody, /data-provider-item/);
  assert.doesNotMatch(patchBody, /innerHTML/);
});

test("provider enable patches API-key controls in place and rolls them back", () => {
  assert.match(settings, /export function patchProviderInteractiveState/);
  const helper = settings.slice(
    settings.indexOf("export function patchProviderInteractiveState"),
    settings.indexOf("export function patchCredentialEye"),
  );
  assert.match(helper, /input\.disabled = disabled/);
  assert.match(helper, /save\.disabled = disabled/);
  assert.match(helper, /remove\.disabled = disabled/);
  assert.doesNotMatch(helper, /innerHTML/);

  const changeHandler = main.slice(main.indexOf("dataset.enable"), main.indexOf('el.id === "cfg-sync"'));
  assert.match(changeHandler, /patchProviderInteractiveState\(id, enabled\)/);
  assert.match(changeHandler, /patchProviderInteractiveState\(id, !enabled\)/);
});

test("typing a key only updates draft and reveal button, never rerenders", () => {
  const start = main.indexOf('document.addEventListener("input"');
  const inputHandler = main.slice(start, main.indexOf('document.addEventListener("change"', start));
  assert.match(inputHandler, /setCredentialDraft\(keyInput, el\.value\)/);
  assert.match(inputHandler, /patchCredentialEye\(keyInput, el\.value\)/);
  assert.doesNotMatch(inputHandler, /renderSettings/);
});

test("the add buttons open the supported-provider catalog", () => {
  assert.match(main, /case "manage-providers"/);
  assert.match(main, /openSettingsCategory\("providers"\)/);
});

test("source preference is optimistic and rolls the native select back on failure", () => {
  const changeHandler = main.slice(main.indexOf("dataset.source"));
  assert.match(changeHandler, /pendingSourcePreferences\.set\(id, preference\)/);
  assert.match(changeHandler, /input\.value = previous \?\? "auto"/);
});

test("failed save/detect report instead of staying silent", () => {
  const saveKey = main.slice(main.indexOf("dataset.savekey"));
  assert.match(saveKey, /showCommandError\(t\("commandFailed"\)\)/);
  const detect = main.slice(main.indexOf('hasAttribute("data-detect")'));
  assert.match(detect, /showCommandError\(t\("commandFailed"\)\)/);
});

test("OAuth errors launch a real provider login command", () => {
  const dash = readFileSync(new URL("../src/views/dash.ts", import.meta.url), "utf8");
  assert.match(dash, /normalized\.action === "login"/);
  assert.match(dash, /data-act="\$\{action\}" data-provider-id=/);
  assert.match(main, /invokeCmd\("start_provider_login", \{ id: btn\.dataset\.providerId \}\)/);
});

test("available updates are installed from the app instead of opening a release page", () => {
  const settings = readFileSync(new URL("../src/views/settings.ts", import.meta.url), "utf8");
  const commands = readFileSync(new URL("../src-tauri/src/commands.rs", import.meta.url), "utf8");
  assert.match(settings, /data-install-update/);
  assert.match(main, /invokeCmd\("install_update"\)/);
  assert.match(commands, /fn published_checksum/);
  // Silent install now runs in a detached PowerShell helper that outlives
  // the app: it waits for the parent PID and inspects the NSIS exit code.
  assert.match(commands, /ArgumentList/);
  assert.match(commands, /\/S/);
  assert.match(commands, /Wait-Process/);
  assert.match(commands, /allow_exit/);
});

test("normal dashboard sections persist their open state per provider", () => {
  const dash = readFileSync(new URL("../src/views/dash.ts", import.meta.url), "utf8");
  assert.match(dash, /dashboard-section:\$\{providerId\}:\$\{section\}/);
  assert.match(dash, /collapsibleSection\(provider\.id, "details"/);
  assert.match(dash, /collapsibleSection\(provider\.id, "actions"/);
  assert.match(main, /details\[data-dashboard-section\]\[data-provider-id\]/);
});

test("an unavailable provider does not invent a source label", () => {
  const dash = readFileSync(new URL("../src/views/dash.ts", import.meta.url), "utf8");
  assert.match(dash, /function connectionHtml/);
  assert.match(dash, /provider\.activeSource[\s\S]*sourceLabel/);
});
