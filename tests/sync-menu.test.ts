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

test("about surfaces the product website, not only GitHub", () => {
  const settings = readFileSync(new URL("../src/views/settings.ts", import.meta.url), "utf8");
  assert.match(settings, /shadelight\.github\.io\/ia-usage-bar/);
  assert.match(settings, /aboutWebsite/);
});

test("sync settings no longer render a passphrase input or manual toggle", () => {
  const settings = readFileSync(new URL("../src/views/settings.ts", import.meta.url), "utf8");
  assert.ok(!settings.includes('data-savesyncpass'), "passphrase save button must be gone");
  assert.ok(!settings.includes('data-forgetsyncpass'), "passphrase forget button must be gone");
  assert.ok(!settings.includes('toggleRow("cfg-sync",'), "manual sync-enable toggle must be gone");
  assert.ok(settings.includes("data-startpairing"), "pairing button must exist");
  assert.ok(settings.includes("data-revokedevice"), "device revoke control must exist");
  assert.ok(settings.includes('class="sync-revoke"'), "unlink button must use the styled sync-revoke class");
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

test("pair-phone checks LAN before starting a pairing, with no passphrase ceremony", () => {
  const flowStart = main.indexOf("async function pairPhoneFlow");
  assert.ok(flowStart > 0, "pairPhoneFlow must exist as a standalone function shared by the menu and the tray");
  const flow = main.slice(flowStart, main.indexOf("\nasync function handleAction", flowStart));
  const lanIdx = flow.indexOf("syncPairNeedsLan");
  const startPairingIdx = flow.indexOf("sync_start_pairing");
  assert.ok(lanIdx > 0, "pair-phone missing the LAN gate");
  assert.ok(startPairingIdx > 0, "pair-phone missing sync_start_pairing");
  assert.ok(lanIdx < startPairingIdx, "pair-phone must check LAN before starting a pairing");
  assert.ok(flow.includes('getElementById("cfg-sync-lan")'), "missing LAN must focus the LAN toggle");
  assert.ok(flow.includes('getElementById("sync-qr")'), "successful pairing must scroll to the QR");
  assert.ok(!flow.includes("hasPassphrase"), "V2 pairing must never gate on a passphrase");
  assert.ok(!flow.includes("sync_set_enabled"), "V2 pairing must never auto-enable a legacy toggle");
});

test("pairing flow guards QR display and export with LAN check, no separate show-QR button", () => {
  const flowStart = main.indexOf("async function pairPhoneFlow");
  assert.ok(flowStart > 0, "pairPhoneFlow must exist");
  const flow = main.slice(flowStart, main.indexOf("\nasync function", flowStart + 1));
  // Verify pairPhoneFlow checks LAN before calling sync_start_pairing
  const lanCheckIdx = flow.indexOf("!status.value.lan");
  const startPairingIdx = flow.indexOf("sync_start_pairing");
  assert.ok(lanCheckIdx > 0 && lanCheckIdx < startPairingIdx, "must check LAN before starting pairing");
  // Verify pairPhoneFlow scrolls to the QR display
  assert.ok(flow.includes('getElementById("sync-qr")'), "pairPhoneFlow must scroll to the QR display");
  // Verify the data-startpairing button handler exists within the click dispatcher
  const clickHandlerStart = main.indexOf('document.addEventListener("click"');
  assert.ok(clickHandlerStart > 0, "click handler must exist");
  const clickHandler = main.slice(clickHandlerStart, main.indexOf("\n  });", clickHandlerStart));
  assert.ok(clickHandler.includes('hasAttribute("data-startpairing")'), "click handler must contain data-startpairing button handler");
  // Verify export button is available
  assert.ok(main.includes('hasAttribute("data-syncexport")'), "sync export button must exist");
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
