import assert from "node:assert/strict";
import test from "node:test";

import type { ProviderStatus, ProviderStatusReason, VendorInfo } from "../src/api.ts";
import {
  normalizeProviderError,
  sanitizeTechnicalDetails,
  shouldShowRecoveryToast,
} from "../src/errors.ts";

const vendor: VendorInfo = {
  id: "anthropic",
  name: "Claude Code",
  short: "CLD",
  authKind: "oauth",
  envKey: null,
  hint: "",
  needsKey: false,
  enabled: true,
  detected: true,
};

const emitted: Array<[Exclude<ProviderStatus, "connected">, ProviderStatusReason]> = [
  ["needs_auth", "missing_credential"],
  ["needs_auth", "invalid_credential"],
  ["needs_auth", "oauth_expired"],
  ["needs_permission", "missing_permission"],
  ["unavailable", "local_service_unavailable"],
  ["unavailable", "network_unavailable"],
  ["error", "rate_limited"],
  ["error", "parse_failed"],
  ["error", "unknown"],
];

test("sign-in action is only offered for vendors whose client the app can launch", () => {
  const expired = { status: "needs_auth" as const, statusReason: "oauth_expired" as const, error: null };
  assert.equal(normalizeProviderError(expired, vendor)?.action, "login");
  const antigravity: VendorInfo = { ...vendor, id: "antigravity", name: "Antigravity", short: "AGY" };
  assert.equal(normalizeProviderError(expired, antigravity)?.action, "retry");
  assert.equal(
    normalizeProviderError({ ...expired, statusReason: "invalid_credential" }, antigravity)?.action,
    "retry",
  );
});

test("every backend status/reason pair resolves to useful copy", () => {
  for (const [status, statusReason] of emitted) {
    const normalized = normalizeProviderError({ status, statusReason, error: "detail" }, vendor);
    assert.ok(normalized?.title);
    assert.ok(normalized?.message);
    assert.ok(normalized?.severity);
  }
  assert.equal(
    normalizeProviderError({ status: "connected", statusReason: null, error: null }, vendor),
    null,
  );
});

test("technical detail sanitizer masks credentials and caps output", () => {
  const raw = [
    "Authorization: Bearer secret-access",
    'api_key="sk-private"',
    'refresh_token: "refresh-private"',
    "cookie=session-private",
    'client_secret: "client-private"',
    "x".repeat(5_000),
  ].join("\n");
  const safe = sanitizeTechnicalDetails(raw);
  assert.ok(!safe.includes("secret-access"));
  assert.ok(!safe.includes("sk-private"));
  assert.ok(!safe.includes("refresh-private"));
  assert.ok(!safe.includes("session-private"));
  assert.ok(!safe.includes("client-private"));
  assert.ok(safe.includes("[REDACTED]"));
  assert.equal(safe.length, 4_000);
});

test("recovery toast fires once only on a real recovery", () => {
  assert.equal(shouldShowRecoveryToast(undefined, "connected"), false);
  assert.equal(shouldShowRecoveryToast("connected", "connected"), false);
  assert.equal(shouldShowRecoveryToast("needs_auth", "connected"), true);
  assert.equal(shouldShowRecoveryToast("unavailable", "connected"), true);
  assert.equal(shouldShowRecoveryToast("error", "error"), false);
});
