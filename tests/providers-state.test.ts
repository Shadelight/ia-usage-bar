// Providers screen: enabled / configured / connection are independent.
// A provider may be enabled + unconfigured + needs_credential without that
// blocking interaction; nothing but an explicit toggle flips `enabled`.

import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import type { ProviderSnapshot, VendorInfo } from "../src/api.ts";
import {
  deriveProviderState,
  formatStatusText,
  isKeyInputDisabled,
} from "../src/provider-state.ts";

function vendor(over: Partial<VendorInfo> = {}): VendorInfo {
  return {
    id: "deepseek",
    name: "DeepSeek",
    short: "DSK",
    authKind: "apikey",
    envKey: "DEEPSEEK_API_KEY",
    hint: "Paste a key.",
    needsKey: true,
    enabled: false,
    detected: false,
    hasCredential: false,
    links: { usageUrl: null, billingUrl: null, statusUrl: null },
    ...over,
  };
}

function snap(over: Partial<ProviderSnapshot> = {}): ProviderSnapshot {
  return {
    id: "deepseek",
    name: "DeepSeek",
    short: "DSK",
    plan: "",
    status: "needs_auth",
    statusReason: "missing_credential",
    stale: false,
    error: null,
    hint: null,
    updatedAt: "",
    quotas: [],
    credits: null,
    productBreakdown: [],
    cost: null,
    lines: [],
    primaryUtilization: null,
    ...over,
  };
}

test("many providers stay enabled simultaneously, credentials or not", () => {
  const ids = ["deepseek", "openrouter", "kimi", "grok", "novita", "moonshot"];
  for (const id of ids) {
    const d = deriveProviderState(vendor({ id, name: id, enabled: true }), snap({ id }), false);
    assert.equal(d.enabled, true, `${id} must remain enabled without credentials`);
    assert.equal(d.connection, "needs_credential");
  }
});

test("enabling without a credential keeps enabled=true and asks for the key", () => {
  const d = deriveProviderState(vendor({ enabled: true }), snap(), false);
  assert.equal(d.enabled, true);
  assert.equal(d.configured, false);
  assert.equal(d.connection, "needs_credential");
  assert.match(formatStatusText(d), /API key/);
});

test("key input is editable as soon as the provider is enabled", () => {
  assert.equal(isKeyInputDisabled(true, false), false);
  assert.equal(isKeyInputDisabled(false, false), true);
  assert.equal(isKeyInputDisabled(true, true), true);
  // Configured/connected state must not appear in this decision: the rule
  // takes only (enabled, saving).
  assert.equal(isKeyInputDisabled.length, 2);
});

test("invalid credential or failed fetch never disables the provider", () => {
  const invalid = deriveProviderState(
    vendor({ enabled: true, hasCredential: true }),
    snap({ status: "needs_auth", statusReason: "invalid_credential" }),
    false,
  );
  assert.equal(invalid.enabled, true);
  assert.equal(invalid.sessionInvalid, true);

  const failed = deriveProviderState(
    vendor({ enabled: true, hasCredential: true }),
    snap({ status: "error", statusReason: "unknown" }),
    false,
  );
  assert.equal(failed.enabled, true);
  assert.equal(failed.connection, "error");

  const down = deriveProviderState(
    vendor({ enabled: true }),
    snap({ status: "unavailable", statusReason: "local_service_unavailable" }),
    false,
  );
  assert.equal(down.enabled, true);
  assert.equal(down.connection, "unavailable");
});

test("deleting a credential returns to needs_credential, still enabled", () => {
  const d = deriveProviderState(vendor({ enabled: true, hasCredential: false }), undefined, false);
  assert.equal(d.enabled, true);
  assert.equal(d.configured, false);
  assert.equal(d.connection, "needs_credential");
});

test("connected rows show channel, disabled rows are gray with text", () => {
  const on = deriveProviderState(
    vendor({ enabled: true, hasCredential: true, authKind: "oauth", id: "anthropic", name: "Claude Code" }),
    snap({ id: "anthropic", status: "connected", statusReason: null }),
    false,
  );
  assert.equal(on.connection, "connected");
  assert.equal(on.dotClass, "status-connected");
  assert.match(formatStatusText(on), /OAuth/);

  const off = deriveProviderState(vendor({ enabled: false }), snap({ status: "connected", statusReason: null }), false);
  assert.equal(off.connection, "disabled");
  assert.equal(off.dotClass, "status-disabled");
  assert.match(formatStatusText(off), /\S/);
});

test("CLI vendors ask for sign-in, not for an API key", () => {
  const d = deriveProviderState(
    vendor({ enabled: true, authKind: "oauth", needsKey: false, id: "anthropic", name: "Claude Code" }),
    snap({ id: "anthropic", status: "needs_auth", statusReason: "missing_credential" }),
    false,
  );
  assert.equal(d.connection, "needs_auth");
  assert.match(formatStatusText(d), /sesi/i);
});

test("providers settings CSS: DS checkbox, scroll area, fixed footer", () => {
  const css = readFileSync(new URL("../src/styles.css", import.meta.url), "utf8");
  const check = css.match(/\.ds-check\s*\{([^}]*)\}/)?.[1] ?? "";
  assert.match(check, /width:\s*17px/);
  assert.match(check, /height:\s*17px/);
  assert.match(css, /\.ds-check:checked::after/);
  assert.match(css, /\.ds-check:focus-visible/);
  const body = css.match(/\.settings-body,\s*\.spend-body\s*\{([^}]*)\}/)?.[1] ?? "";
  assert.match(body, /overflow-y:\s*auto/);
  assert.match(css, /\.prov-chev\.open/);
  assert.match(css, /\.status-disabled/);
  const panel = css.match(/\.panel\s*\{([^}]*)\}/)?.[1] ?? "";
  assert.match(panel, /flex-direction:\s*column/);
  const html = readFileSync(new URL("../index.html", import.meta.url), "utf8");
  assert.match(html, /<footer class="foot">/);
  assert.match(html, /id="settings-body"/);
});
