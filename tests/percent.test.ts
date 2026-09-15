import assert from "node:assert/strict";
import test from "node:test";

import { ROUNDING_VECTORS, normalizePercentageMode, summaryPercents } from "../src/percent.ts";

test("summaryPercents matches the cross-platform rounding contract", () => {
  for (const [exact, used, remaining] of ROUNDING_VECTORS) {
    assert.deepEqual(summaryPercents(exact), { used, remaining }, `usedExact=${exact}`);
  }
});

test("available is accepted as an alias of remaining", () => {
  assert.equal(normalizePercentageMode("available"), "remaining");
  assert.equal(normalizePercentageMode("remaining"), "remaining");
  assert.equal(normalizePercentageMode("used"), "used");
});
