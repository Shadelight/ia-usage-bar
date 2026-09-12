import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { activeItemScrollDelta, cachePresentation, elapsedLabel, horizontalWheelDelta } from "../src/layout.ts";

test("vertical wheel input becomes horizontal movement", () => {
  assert.equal(horizontalWheelDelta(0, 120), 120);
  assert.equal(horizontalWheelDelta(-80, 10), -80);
});

test("selected item scrolls only when outside the viewport", () => {
  assert.equal(activeItemScrollDelta({ left: 0, right: 300 }, { left: 80, right: 180 }), 0);
  assert.equal(activeItemScrollDelta({ left: 0, right: 300 }, { left: 280, right: 360 }), 60);
  assert.equal(activeItemScrollDelta({ left: 20, right: 300 }, { left: -10, right: 40 }), -30);
});

test("cached data remains visible while a provider refreshes", () => {
  assert.equal(cachePresentation(true, true), "cached");
  assert.equal(cachePresentation(false, true), "empty");
});

test("last-update age is derived from the snapshot timestamp", () => {
  assert.deepEqual(elapsedLabel("2026-09-11T12:00:00Z", Date.parse("2026-09-11T12:03:20Z")), { count: 3, unit: "minutes" });
});

test("stale data distinguishes the failed attempt from the valid-data timestamp", () => {
  const dash = readFileSync(new URL("../src/views/dash.ts", import.meta.url), "utf8");
  assert.match(dash, /refreshFailedAttempt/);
  assert.match(dash, /lastAttemptAt/);
  assert.match(dash, /staleDataFrom/);
});

test("the native window is the only owner of the outer rounded chrome", () => {
  const css = readFileSync(new URL("../src/styles.css", import.meta.url), "utf8");
  const panelRule = css.match(/\.panel\s*\{([^}]*)\}/)?.[1] ?? "";

  assert.match(panelRule, /border-radius:\s*0\s*;/);
  assert.match(panelRule, /border:\s*0\s*;/);
  assert.match(panelRule, /box-shadow:\s*none\s*;/);
  assert.doesNotMatch(css, /--radius\s*:/);
});
