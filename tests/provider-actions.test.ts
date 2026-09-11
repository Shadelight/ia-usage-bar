import assert from "node:assert/strict";
import test from "node:test";

import type { ProviderSnapshot, VendorInfo } from "../src/api.ts";
import {
  actionIconSvg,
  buildSanitizedDiagnosis,
  getCompactQuickActions,
  getProviderActions,
  getProviderCliCommand,
} from "../src/provider-actions.ts";

const fullVendor: VendorInfo = {
  id: "anthropic",
  name: "Claude Code",
  short: "CLD",
  authKind: "oauth",
  envKey: null,
  hint: "",
  needsKey: false,
  enabled: true,
  detected: true,
  links: {
    usageUrl: "https://claude.ai/settings/usage",
    billingUrl: "https://claude.ai/settings/billing",
    statusUrl: "https://status.anthropic.com",
    docsUrl: "https://docs.anthropic.com",
    appUrl: "https://claude.ai",
  },
};

const minimalVendor: VendorInfo = {
  id: "custom_ai",
  name: "Custom AI",
  short: "CAI",
  authKind: "apikey",
  envKey: "CUSTOM_KEY",
  hint: "",
  needsKey: true,
  enabled: true,
  detected: false,
  links: {
    usageUrl: null,
    billingUrl: null,
    statusUrl: null,
    docsUrl: null,
    appUrl: null,
  },
};

const sampleSnapshot: ProviderSnapshot = {
  id: "anthropic",
  name: "Claude Code",
  short: "CLD",
  plan: "Max",
  status: "connected",
  statusReason: null,
  stale: false,
  error: null,
  hint: null,
  updatedAt: "2026-09-11T18:20:00Z",
  quotas: [
    {
      id: "session",
      label: "Sesión",
      windowType: "session",
      usedPercent: 88,
      remainingPercent: 12,
      usedAmount: null,
      limitAmount: null,
      unit: "percent",
      resetAt: "2026-09-11T19:03:00Z",
      resetInSeconds: 2580,
      resetStatus: "known",
      temporaryMultiplier: null,
      temporaryExpiresAt: null,
      source: "oauth",
      fetchedAt: "2026-09-11T18:20:00Z",
      stale: false,
    },
    {
      id: "weekly",
      label: "Semanal",
      windowType: "weekly",
      usedPercent: 12,
      remainingPercent: 88,
      usedAmount: null,
      limitAmount: null,
      unit: "percent",
      resetAt: "2026-09-14T20:30:00Z",
      resetInSeconds: 267000,
      resetStatus: "known",
      temporaryMultiplier: null,
      temporaryExpiresAt: null,
      source: "oauth",
      fetchedAt: "2026-09-11T18:20:00Z",
      stale: false,
    },
  ],
  credits: {
    remaining: 250,
    resetsAvailable: 5,
  },
  productBreakdown: [],
  cost: {
    today: 1.25,
    week: 10.5,
    thirtyDays: 45.0,
    month: 45.0,
  },
  lines: [],
  primaryUtilization: 88,
};

test("sanitized diagnosis formats cleanly and contains no secrets", () => {
  const diag = buildSanitizedDiagnosis(sampleSnapshot, fullVendor);

  assert.ok(diag.includes("Claude Code"));
  assert.ok(diag.includes("Status: connected"));
  assert.ok(diag.includes("Plan: Max"));
  assert.ok(diag.includes("88% used"));
  assert.ok(diag.includes("Credits: 250"));
  assert.ok(diag.includes("Month: $45.00"));
  assert.ok(diag.includes("IA Usage: 0.2.0"));

  // Ensure absolutely no secret tokens or keys
  assert.ok(!diag.includes("sk-"));
  assert.ok(!diag.includes("Bearer"));
  assert.ok(!diag.includes("token"));
  assert.ok(!diag.includes("password"));
});

test("sanitized diagnosis gracefully handles missing or partial data", () => {
  const partialSnapshot: ProviderSnapshot = {
    ...sampleSnapshot,
    plan: "",
    status: "needs_auth",
    statusReason: "missing_credential",
    quotas: [],
    credits: null,
    cost: null,
  };
  const diag = buildSanitizedDiagnosis(partialSnapshot, minimalVendor);

  assert.ok(diag.includes("Custom AI"));
  assert.ok(diag.includes("Status: needs_auth (missing_credential)"));
  assert.ok(diag.includes("Plan: —"));
  assert.ok(diag.includes("IA Usage: 0.2.0"));
});

test("getProviderActions resolves all available external links and standard actions", () => {
  const actions = getProviderActions(fullVendor);

  const urls = actions.filter((a) => a.kind === "external").map((a) => a.url);
  assert.deepEqual(urls, [
    "https://claude.ai/settings/usage",
    "https://claude.ai/settings/billing",
    "https://status.anthropic.com",
    "https://docs.anthropic.com",
    "https://claude.ai",
  ]);

  const configureAction = actions.find((a) => a.kind === "configure");
  assert.ok(configureAction);
  assert.equal(configureAction?.labelKey, "configureProvider");
  assert.equal(configureAction?.icon, "gear");
  assert.equal(configureAction?.indicator, "chevron-right");

  const diagAction = actions.find((a) => a.kind === "diagnosis");
  assert.ok(diagAction);
  assert.equal(diagAction?.labelKey, "copyDiagnosis");
  assert.equal(diagAction?.icon, "clipboard");
  assert.equal(diagAction?.indicator, "copy");
});

test("getProviderActions excludes unavailable links for providers without URLs", () => {
  const actions = getProviderActions(minimalVendor);

  const externalActions = actions.filter((a) => a.kind === "external");
  assert.equal(externalActions.length, 0);

  // Internal actions (configure, diagnosis) are still available
  assert.ok(actions.some((a) => a.kind === "configure"));
  assert.ok(actions.some((a) => a.kind === "diagnosis"));
});

test("getProviderCliCommand formats dedicated CLI command without emojis", () => {
  const cmd = getProviderCliCommand(fullVendor);
  assert.equal(cmd, "iausage usage anthropic");
  assert.ok(!/\p{Extended_Pictographic}/u.test(cmd));
});

test("compact mode quick actions are capped at a maximum of two", () => {
  const fullCompact = getCompactQuickActions(fullVendor);
  assert.equal(fullCompact.length, 2);
  assert.equal(fullCompact[0].labelKey, "panel");
  assert.equal(fullCompact[0].url, "https://claude.ai/settings/usage");
  assert.equal(fullCompact[1].labelKey, "configure");

  const minimalCompact = getCompactQuickActions(minimalVendor);
  assert.equal(minimalCompact.length, 1);
  assert.equal(minimalCompact[0].labelKey, "configure");
});

test("actionIconSvg provides consistent SVGs and contains no Unicode emojis", () => {
  const icons = [
    "dashboard",
    "billing",
    "status",
    "docs",
    "app",
    "gear",
    "clipboard",
    "terminal",
    "external-link",
    "chevron-right",
    "copy",
    "warning",
    "refresh",
    "check",
  ];

  for (const name of icons) {
    const svg = actionIconSvg(name, 14);
    assert.ok(svg.startsWith("<svg"), `${name} must start with <svg`);
    assert.ok(svg.endsWith("</svg>"), `${name} must end with </svg>`);
    assert.ok(svg.includes('width="14"'), `${name} must have width="14"`);
    assert.ok(svg.includes('height="14"'), `${name} must have height="14"`);
    assert.ok(svg.includes('stroke="currentColor"'), `${name} must use currentColor`);
    assert.ok(svg.includes('stroke-width="2"'), `${name} must have consistent stroke-width`);
    assert.ok(!svg.includes("↗"), `${name} must not contain ↗`);
    assert.ok(!svg.includes("›"), `${name} must not contain ›`);
    assert.ok(!svg.includes("📋"), `${name} must not contain 📋`);
    assert.ok(!svg.includes("⚠"), `${name} must not contain ⚠`);
    assert.ok(!/\p{Extended_Pictographic}/u.test(svg), `${name} must not contain emojis`);
  }
});
