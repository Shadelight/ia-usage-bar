// Sync keyring fix + head menu as navigation + pair-phone preconditions.
//
// Backend: sync passphrase helpers must use the NUL-tolerant keyring reader
// (same Credential Manager quirk already fixed for API keys). Frontend: the
// head menu navigates to Settings sections; pair-phone checks passphrase,
// LAN and enabled state in that order before generating a QR (a localhost
// QR without LAN would be useless).
import assert from "node:assert/strict";
import test from "node:test";
import { readFileSync } from "node:fs";

import { I18N } from "../src/i18n.ts";
import { actionIconSvg } from "../src/provider-actions.ts";

const main = readFileSync(new URL("../src/main.ts", import.meta.url), "utf8");
const sync = readFileSync(
  new URL("../src-tauri/../crates/iausage-core/src/sync.rs", import.meta.url),
  "utf8",
);
const config = readFileSync(
  new URL("../src-tauri/../crates/iausage-core/src/config.rs", import.meta.url),
  "utf8",
);

test("keyring reader is reusable and sync uses it everywhere", () => {
  assert.match(config, /pub\(crate\) fn read_keyring_entry/);
  assert.match(sync, /read_keyring_entry/);
  const code = sync.replace(/\/\/.*/g, "");
  assert.ok(!code.includes("get_password"), "sync must not call get_password directly");
  assert.match(config, /read_keyring_entry\(&entry\)\.is_some\(\)/);
});

test("sync menu i18n keys exist in both languages", () => {
  for (const lang of ["es", "en"] as const) {
    assert.ok(I18N[lang].syncPhoneMenu?.length > 0, `${lang}.syncPhoneMenu missing`);
    assert.ok(I18N[lang].syncPairNeedsLan?.length > 0, `${lang}.syncPairNeedsLan missing`);
  }
  assert.notEqual(I18N.es.syncPhoneMenu, "syncPhoneMenu");
  assert.notEqual(I18N.en.syncPairNeedsLan, "syncPairNeedsLan");
});

test("menu icons exist and render svg", () => {
  for (const icon of ["bell", "phone", "folder", "settings"]) {
    const svg = actionIconSvg(icon, 14);
    assert.ok(svg.includes("<svg"), `icon ${icon} renders nothing`);
  }
});

test("head menu navigates instead of toggling", () => {
  const menu = main.slice(main.indexOf("function paintHeadMenu"), main.indexOf("function paintDash"));
  assert.ok(!menu.includes("toggle-notif"), "menu must not toggle notifications");
  assert.ok(!menu.includes("openLogs"), "menu must not open the logs folder directly");
  assert.ok(!menu.includes("providerPanel"), "provider panel belongs in provider actions");
  assert.ok(!menu.includes("serviceStatus"), "service status belongs in provider actions");
  for (const act of ["settings-notifications", "spend", "pair-phone", "settings-data", "settings-general"]) {
    assert.ok(menu.includes(`data-act="${act}"`), `menu missing ${act}`);
  }
  assert.ok(menu.includes('t("syncPhoneMenu")'), "menu must use the syncPhoneMenu label");
  assert.ok(menu.includes('t("settingsData")'), "menu must use the settingsData label");
});

test("head menu rows actually render the bell/phone/folder/settings icons, not just text", () => {
  const menu = main.slice(main.indexOf("function paintHeadMenu"), main.indexOf("function paintDash"));
  for (const [act, icon] of [
    ["settings-notifications", "bell"],
    ["pair-phone", "phone"],
    ["settings-data", "folder"],
    ["settings-general", "settings"],
  ] as const) {
    const rowStart = menu.indexOf(`data-act="${act}"`);
    assert.ok(rowStart > 0, `menu missing row for ${act}`);
    const row = menu.slice(rowStart, menu.indexOf("`,", rowStart));
    assert.ok(row.includes(`actionIconSvg("${icon}"`), `${act} row must render the ${icon} icon, not just a label`);
  }
});

test("tray-cmd pair-phone triggers the same flow as the in-app menu, and settings deep-links are not reset afterwards", () => {
  const listenerStart = main.indexOf('listen<string>("tray-cmd"');
  const listener = main.slice(listenerStart, main.indexOf("});", listenerStart));
  assert.ok(listener.includes('cmd === "pair-phone"'), "tray-cmd must handle the pair-phone payload");
  assert.ok(listener.includes("pairPhoneFlow()"), "tray-cmd pair-phone must call the shared pairPhoneFlow, not duplicate it");
  assert.ok(!listener.includes('handleAction("settings")'), "tray-cmd must not reset settingsCategory back to general after applying the requested category");
});

test("pair-phone checks passphrase, LAN and enabled before pairing", () => {
  const flowStart = main.indexOf("async function pairPhoneFlow");
  assert.ok(flowStart > 0, "pairPhoneFlow must exist as a standalone function shared by the menu and the tray");
  const flow = main.slice(flowStart, main.indexOf("\nasync function handleAction", flowStart));
  const passIdx = flow.indexOf("hasPassphrase");
  const lanIdx = flow.indexOf("syncPairNeedsLan");
  const enabledIdx = flow.indexOf("sync_set_enabled");
  const pairingIdx = flow.indexOf("sync_get_pairing");
  for (const [name, idx] of [["hasPassphrase", passIdx], ["lan gate", lanIdx], ["auto-enable", enabledIdx], ["pairing", pairingIdx]] as const) {
    assert.ok(idx > 0, `pair-phone missing ${name}`);
  }
  assert.ok(passIdx < lanIdx && lanIdx < enabledIdx && enabledIdx < pairingIdx, "pair-phone must check passphrase, then LAN, then enabled, then pair");
  assert.ok(flow.includes("{ lan: true }"), "pairing must request lan:true, never the unchecked toggle");
  assert.ok(flow.includes("serverRunning"), "pairing must wait for a running LAN server");
  assert.ok(flow.includes('getElementById("sync-pass")'), "missing passphrase must focus the secret input");
  assert.ok(flow.includes('getElementById("cfg-sync-lan")'), "missing LAN must focus the LAN toggle");
  assert.ok(flow.includes('getElementById("sync-qr")'), "successful pairing must scroll to the QR");
});

test("cfg-sync reverts the checkbox when the backend refuses", () => {
  const handler = main.slice(main.indexOf('el.id === "cfg-sync"'));
  assert.ok(handler.includes("input.checked = !enabled"), "cfg-sync must revert on failure");
});

test("Show QR uses the same guarded mobile pairing flow", () => {
  const start = main.indexOf('hasAttribute("data-syncqr")');
  const handler = main.slice(start, main.indexOf('hasAttribute("data-syncexport")', start));
  assert.match(handler, /await pairPhoneFlow\(\)/);
  assert.doesNotMatch(handler, /lanToggle|lan: false/);
});

test("footer uses the Tauri package version with a browser fallback", () => {
  const html = readFileSync(new URL("../index.html", import.meta.url), "utf8");
  assert.match(html, /id="app-version">IA Usage</);
  assert.match(main, /async function paintAppVersion/);
  assert.match(main, /getVersion/);
  assert.match(main, /IA Usage · v\$\{/);
});

test("CLI diagnostics and exact installer PATH checks are wired", () => {
  const settings = readFileSync(new URL("../src/views/settings.ts", import.meta.url), "utf8");
  const commands = readFileSync(new URL("../src-tauri/src/commands.rs", import.meta.url), "utf8");
  const hooks = readFileSync(new URL("../src-tauri/windows/hooks.nsh", import.meta.url), "utf8");
  const releaseConfig = readFileSync(new URL("../src-tauri/tauri.release.conf.json", import.meta.url), "utf8");
  for (const command of ["cli_install_status", "repair_cli_path", "cli_test"]) {
    assert.ok(commands.includes(`fn ${command}`), `${command} backend missing`);
    assert.ok(`${main}\n${settings}`.includes(`"${command}"`), `${command} frontend wiring missing`);
  }
  assert.match(settings, /data-repair-cli/);
  assert.match(hooks, /IfFileExists "\$INSTDIR\\resources\\bin\\iausage\.exe"/);
  assert.match(hooks, /Function AddCliToPath/);
  assert.doesNotMatch(hooks, /\$\{StrStr\}/);
  assert.match(releaseConfig, /"resources\/bin\/iausage\.exe"/);
});
