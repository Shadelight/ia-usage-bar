import assert from "node:assert/strict";
import test from "node:test";

import { formatResetAbsolute, formatResetRelative } from "../src/reset-format.ts";

const TIME_ZONE = "America/La_Paz";
const NOW = new Date("2026-09-11T09:05:00-04:00").getTime();

test("formats a relative session reset without fetching again", () => {
  assert.equal(formatResetRelative("2026-09-11T13:40:00-04:00", NOW, "es"), "4 h 35 min");
});

test("formats relative weekly resets with days, hours and leftover minutes", () => {
  assert.equal(formatResetRelative("2026-09-16T05:00:00-04:00", NOW, "es"), "4 d 19 h 55 min");
  assert.equal(formatResetRelative("2026-09-16T05:00:00-04:00", NOW, "es", true), "4 d 19 h");
});

test("formats same-day and future absolute resets in the requested timezone", () => {
  assert.equal(formatResetAbsolute("2026-09-11T13:40:00-04:00", "es", NOW, TIME_ZONE), "Hoy, 13:40");
  assert.equal(formatResetAbsolute("2026-09-16T05:00:00-04:00", "es", NOW, TIME_ZONE), "16 sept, 05:00");
});

test("unknown or invalid resets stay empty so resetStatus can explain them", () => {
  assert.equal(formatResetRelative(null, NOW, "es"), "");
  assert.equal(formatResetAbsolute("not-a-date", "es", NOW, TIME_ZONE), "");
});
