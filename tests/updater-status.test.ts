// Updater UX: real backend phases mapped to i18n, with the legacy
// `downloadingUpdate` string kept as a compatibility fallback.
import assert from "node:assert/strict";
import test from "node:test";
import { readFileSync } from "node:fs";

import { I18N } from "../src/i18n.ts";

const main = readFileSync(new URL("../src/main.ts", import.meta.url), "utf8");
const commands = readFileSync(
  new URL("../src-tauri/src/commands.rs", import.meta.url),
  "utf8",
);

const PHASE_KEYS: Record<string, string> = {
  downloading: "updateDownloading",
  verifying: "updateVerifying",
  preparing: "updatePreparing",
  restarting: "updateRestarting",
};

test("updater phase keys exist in both languages and differ from fallback", () => {
  for (const [phase, key] of Object.entries(PHASE_KEYS)) {
    for (const lang of ["es", "en"] as const) {
      const text = I18N[lang][key];
      assert.ok(text && text.length > 0, `${lang}.${key} missing (phase ${phase})`);
      assert.notEqual(text, key, `${lang}.${key} is untranslated`);
    }
    // The legacy string stays as fallback and must not be reused verbatim.
    assert.notEqual(I18N.es[key], I18N.es.downloadingUpdate, `es.${key} duplicates fallback`);
  }
  assert.ok(I18N.es.downloadingUpdate.length > 0, "fallback downloadingUpdate removed");
  assert.ok(I18N.en.downloadingUpdate.length > 0, "fallback downloadingUpdate removed");
});

test("frontend listens to updater-status and maps phases to i18n", () => {
  assert.ok(
    main.includes('listen<string>("updater-status"'),
    "main.ts does not subscribe to updater-status",
  );
  for (const [phase, key] of Object.entries(PHASE_KEYS)) {
    assert.ok(main.includes(phase), `main.ts does not map phase ${phase}`);
    assert.ok(main.includes(`"${key}"`) || main.includes(`'${key}'`) || main.includes(key), `main.ts does not use i18n key ${key}`);
  }
});

test("backend emits stable snake_case phases and guards exit", () => {
  for (const phase of Object.keys(PHASE_KEYS)) {
    assert.ok(
      commands.includes(`"${phase}"`) || commands.includes(phase),
      `commands.rs missing phase ${phase}`,
    );
  }
  assert.ok(commands.includes("UpdaterPhase"), "UpdaterPhase enum missing");
  assert.ok(commands.includes("ps_quote"), "ps_quote helper missing");
  assert.ok(commands.includes("Wait-Process"), "helper must wait for parent PID");
  assert.ok(commands.includes("ExitCode"), "helper must inspect installer exit code");
  assert.ok(commands.includes("allow_exit"), "install_update must set allow_exit before exit");
  assert.ok(commands.includes("current_exe"), "install_update must use current_exe");
});
