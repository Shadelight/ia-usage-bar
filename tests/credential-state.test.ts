// Credential block: optimistic presence, scoped validation, API-key wording,
// and the Obtener clave / Crear cuenta links.
//
// Covers the OpenCode Go desync (saved key repainted as "missing") plus the
// five follow-up corrections: validating state ownership, save→validate
// split, compare-first reconciliation, 401 wording per auth kind, and the
// stale-detected resurrection bug (the Rust side lives in
// providers::tests).

import assert from "node:assert/strict";
import test from "node:test";
import { readFileSync } from "node:fs";

import type { ProviderSnapshot, VendorInfo } from "../src/api.ts";
import { deriveProviderState } from "../src/provider-state.ts";
import { normalizeProviderError } from "../src/errors.ts";
import { I18N } from "../src/i18n.ts";

const main = readFileSync(new URL("../src/main.ts", import.meta.url), "utf8");
const settings = readFileSync(new URL("../src/views/settings.ts", import.meta.url), "utf8");
const commands = readFileSync(new URL("../src-tauri/src/commands.rs", import.meta.url), "utf8");

function keyVendor(over: Partial<VendorInfo> = {}): VendorInfo {
  return {
    id: "opencode_go",
    name: "OpenCode Go",
    short: "OCG",
    authKind: "apikey",
    envKey: "OPENCODE_GO_API_KEY",
    hint: "Paste a key.",
    needsKey: true,
    enabled: true,
    detected: false,
    hasCredential: false,
    links: {
      usageUrl: null,
      billingUrl: null,
      statusUrl: null,
      apiKeyUrl: "https://opencode.ai/auth",
      signupUrl: "https://opencode.ai/",
    },
    strategies: ["api"],
    sourcePreference: null,
    ...over,
  };
}

function oauthVendor(over: Partial<VendorInfo> = {}): VendorInfo {
  return {
    id: "anthropic",
    name: "Claude Code",
    short: "CLD",
    authKind: "oauth",
    envKey: null,
    hint: "Run claude.",
    needsKey: false,
    enabled: true,
    detected: true,
    hasCredential: true,
    links: { usageUrl: null, billingUrl: null, statusUrl: null },
    strategies: ["oauth"],
    sourcePreference: null,
    ...over,
  };
}

function invalidSnap(over: Partial<ProviderSnapshot> = {}): ProviderSnapshot {
  return {
    id: "opencode_go",
    name: "OpenCode Go",
    short: "OCG",
    plan: "",
    status: "needs_auth",
    statusReason: "invalid_credential",
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

test("a rejected API key reads as invalid credential, not invalid session", () => {
  const api = deriveProviderState(keyVendor({ hasCredential: true }), invalidSnap(), false);
  assert.equal(api.statusKey, "statusCredentialInvalid");
  assert.equal(api.sessionInvalid, false);

  const oauth = deriveProviderState(
    oauthVendor(),
    invalidSnap({ id: "anthropic" }),
    false,
  );
  assert.equal(oauth.statusKey, "statusSessionInvalid");
  assert.equal(oauth.sessionInvalid, true);
});

test("the error card sends API-key failures to replace the credential", () => {
  const api = normalizeProviderError(
    { status: "needs_auth", statusReason: "invalid_credential", error: null },
    keyVendor({ hasCredential: true }),
  );
  assert.equal(api?.action, "configure_credentials");
  assert.match(api?.title ?? "", /credencial/i);

  const oauth = normalizeProviderError(
    { status: "needs_auth", statusReason: "invalid_credential", error: null },
    oauthVendor(),
  );
  assert.equal(oauth?.action, "login");
});

test("credential i18n keys exist in both locales", () => {
  for (const key of [
    "getApiKey",
    "signUp",
    "replaceCredential",
    "replaceKeyPlaceholder",
    "showCredential",
    "hideCredential",
    "validatingCredential",
    "statusCredentialInvalid",
    "errorInvalidApiKeyTitle",
    "errorInvalidApiKeyMessage",
  ]) {
    assert.ok(I18N.es[key], `es.${key} must exist`);
    assert.ok(I18N.en[key], `en.${key} must exist`);
  }
  assert.match(I18N.es.statusCredentialInvalid, /Credencial no válida/);
});

test("validating state lives in settings.ts so render code avoids a cycle", () => {
  assert.match(settings, /const validatingCredentials = new Set<string>\(\)/);
  assert.match(settings, /export function setCredentialValidating/);
  assert.match(settings, /export function isCredentialValidating/);
  assert.match(main, /setCredentialValidating\(id, true\)/);
  assert.doesNotMatch(settings, /from "\.\.\/main/);
});

test("save persists first and validates after, never inside the command", () => {
  const saveStart = commands.indexOf("pub(crate) fn save_api_key");
  const deleteStart = commands.indexOf("pub(crate) fn delete_api_key");
  const saveBody = commands.slice(saveStart, deleteStart);
  assert.match(saveBody, /emit_dashboard\(&app\)/);
  assert.doesNotMatch(saveBody, /do_refresh/);
  const afterDelete = commands.slice(deleteStart);
  const deleteEnd = afterDelete.indexOf("#[tauri::command]", 1);
  const deleteBody = afterDelete.slice(0, deleteEnd === -1 ? undefined : deleteEnd);
  assert.match(deleteBody, /emit_dashboard\(&app\)/);
  assert.doesNotMatch(deleteBody, /do_refresh/);

  const saveHandler = main.slice(main.indexOf("btn.dataset.savekey"));
  assert.match(saveHandler, /input\?\.value\.trim\(\)/);
  assert.match(saveHandler, /if \(!key\) return;/);
  assert.match(saveHandler, /pendingCredentialPresence\.set\(id, true\)/);
  assert.match(saveHandler, /setCredentialValidating\(id, true\)/);
  assert.match(
    saveHandler,
    /invokeCmd\("save_api_key"[\s\S]*?invokeCmd\("refresh_provider", \{ id \}\)/,
    "refresh_provider must run after save_api_key resolves",
  );
  const deleteHandler = main.slice(main.indexOf("btn.dataset.delkey"));
  assert.match(deleteHandler, /pendingCredentialPresence\.set\(id, false\)/);
  assert.match(deleteHandler, /setCredentialValidating\(id, false\)/);
  assert.match(deleteHandler, /invokeCmd\("refresh_provider", \{ id \}\)/);
});

test("reconciliation compares before overwriting, never assigns first", () => {
  const reconcile = main.slice(main.indexOf("async function applyDashboard"));
  assert.match(reconcile, /vendor\.hasCredential === pendingCredential/);
  assert.match(reconcile, /pendingCredentialPresence\.delete\(vendor\.id\)/);
  assert.match(reconcile, /pendingCredentialPresenceExpiry/);
  assert.match(reconcile, /performance\.now\(\)/);
  assert.match(reconcile, /vendor\.enabled === pendingEnabled/);
  assert.match(reconcile, /vendor\.sourcePreference === pendingSource/);
  assert.match(reconcile, /credentialValidationObserved/);
  assert.match(
    reconcile,
    /credentialValidationObserved\.has\(vendor\.id\)/,
    "validating must only end after its loading state was observed",
  );
});

test("credential header shows key links and the eye only while typing", () => {
  assert.match(settings, /function credentialLinks/);
  assert.match(settings, /getProviderActions\(vendor\)/);
  assert.match(settings, /action\.id === "api-key"/);
  assert.match(settings, /action\.id === "signup"/);
  assert.match(settings, /t\(action\.labelKey\)/);
  assert.match(settings, /class="credential-head"/);
  assert.match(settings, /class="credential-input-wrap"/);
  assert.match(settings, /data-toggle-key/);
  assert.match(settings, /t\("replaceCredential"\)/);
  assert.match(settings, /t\("replaceKeyPlaceholder"\)/);
  const eyeBranch = settings.slice(settings.indexOf("const eye = draft"));
  assert.match(eyeBranch, /const eye = draft/);
  assert.match(main, /btn\.dataset\.toggleKey/);
});

test("API-key providers never show an OAuth sign-in state", () => {
  const unreadable = deriveProviderState(
    keyVendor({ hasCredential: true }),
    invalidSnap({ statusReason: "missing_credential" }),
    false,
  );
  assert.equal(unreadable.statusKey, "statusCredentialUnreadable");
  assert.notEqual(unreadable.statusKey, "statusNeedLogin");
});

test("patchSettings keeps text-only mutation while validating", () => {
  const patchBody = settings.slice(settings.indexOf("export function patchSettings"));
  assert.match(patchBody, /isCredentialValidating\(vendor\.id\)/);
  assert.match(patchBody, /t\("validatingCredential"\)/);
  assert.doesNotMatch(patchBody, /innerHTML/);
});
