import assert from "node:assert/strict";
import test from "node:test";

import type { UsageQuota } from "../src/api.ts";
import { groupsFromQuotas } from "../src/quota-groups.ts";

function quota(partial: Partial<UsageQuota> & { id: string; label: string }): UsageQuota {
  return {
    windowType: "weekly",
    usedPercent: 0,
    remainingPercent: 100,
    usedAmount: null,
    limitAmount: null,
    unit: "percent",
    resetAt: null,
    resetInSeconds: null,
    resetStatus: "not_provided",
    temporaryMultiplier: null,
    temporaryExpiresAt: null,
    source: "oauth",
    fetchedAt: "2026-09-15T00:00:00Z",
    stale: false,
    visible: "always",
    ...partial,
  };
}

test("groupsFromQuotas keeps Antigravity 2x2 and skips details", () => {
  const groups = groupsFromQuotas([
    quota({ id: "gemini_models_weekly", label: "weekly", groupId: "gemini_models", groupLabel: "Gemini Models", models: ["Gemini Flash", "Gemini Pro"], usedPercent: 5.48 }),
    quota({ id: "gemini_models_5h", label: "5h", windowType: "5h", groupId: "gemini_models", groupLabel: "Gemini Models", usedPercent: 14.88 }),
    quota({ id: "claude_gpt_models_weekly", label: "weekly", groupId: "claude_gpt_models", groupLabel: "Claude + GPT", usedPercent: 0 }),
    quota({ id: "claude_gpt_models_5h", label: "5h", windowType: "5h", groupId: "claude_gpt_models", groupLabel: "Claude + GPT", usedPercent: 0 }),
    quota({ id: "total", label: "Total", visible: "details", usedPercent: 40 }),
  ]);
  assert.equal(groups.length, 2);
  assert.equal(groups[0].id, "gemini_models");
  assert.equal(groups[0].quotas.length, 2);
  assert.deepEqual(groups[0].models, ["Gemini Flash", "Gemini Pro"]);
  assert.equal(groups[1].label, "Claude + GPT");
});
