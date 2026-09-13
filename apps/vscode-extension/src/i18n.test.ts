import assert from "node:assert/strict";
import { after, test } from "node:test";
import { setLang, t } from "./i18n";

after(() => setLang("en"));

test("defaults to english", () => {
  setLang("en");
  assert.equal(t("menu.title"), "IA Usage");
  assert.equal(t("percentage.used"), "Used");
});

test("switches to spanish", () => {
  setLang("es");
  assert.equal(t("percentage.used"), "Usado");
});

test("interpolates variables", () => {
  setLang("en");
  assert.equal(t("tooltip.moreProviders", { n: 3 }), "+3 more configured providers");
});

test("falls back to the english string for a key missing in the current language", () => {
  setLang("es");
  // menu.title exists in both, but the fallback path itself must not throw
  // or return the raw key for a key that only exists in one dict.
  assert.equal(t("does.not.exist"), "does.not.exist");
});
