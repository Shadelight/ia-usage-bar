import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

import { COPY, resolveLang } from "./app.js";

const root = dirname(fileURLToPath(import.meta.url));
const html = readFileSync(join(root, "index.html"), "utf8");

test("ES and EN expose the same i18n keys", () => {
  const en = Object.keys(COPY.en).sort();
  const es = Object.keys(COPY.es).sort();
  assert.deepEqual(es, en);
});

test("every data-i18n key exists in COPY", () => {
  const keys = [...html.matchAll(/data-i18n(?:-alt|-aria)?="([^"]+)"/g)].map((match) => match[1]);
  assert.ok(keys.length > 0);
  for (const key of keys) {
    assert.ok(COPY.en[key], `missing EN key ${key}`);
    assert.ok(COPY.es[key], `missing ES key ${key}`);
  }
});

test("platform cards do not hardcode a version", () => {
  const download = html.slice(html.indexOf('id="download"'), html.indexOf('id="providers"'));
  assert.doesNotMatch(download, /v0\.\d/);
  assert.doesNotMatch(download, /v1\.\d/);
});

test("download CTAs have a real https fallback, never #", () => {
  const hrefs = [...html.matchAll(/<a\b[^>]*data-asset="[^"]+"[^>]*>/g)].map((match) => match[0]);
  assert.ok(hrefs.length >= 6);
  for (const tag of hrefs) {
    assert.match(tag, /href="https:\/\/github\.com\/Shadelight\/ia-usage-bar\/releases\/latest"/);
    assert.doesNotMatch(tag, /href="#"/);
    assert.doesNotMatch(tag, /javascript:void/);
  }
});

test("Open VSX is linked and Marketplace is omitted while unpublished", () => {
  assert.match(html, /https:\/\/open-vsx\.org\/extension\/shadelightdev\/ia-usage/);
  assert.doesNotMatch(html, /marketplace\.visualstudio\.com/);
});

test("sitemap exists and matches robots.txt", () => {
  const robots = readFileSync(join(root, "robots.txt"), "utf8");
  const sitemap = readFileSync(join(root, "sitemap.xml"), "utf8");
  assert.match(robots, /Sitemap: https:\/\/shadelight\.github\.io\/ia-usage-bar\/sitemap\.xml/);
  assert.match(sitemap, /<loc>https:\/\/shadelight\.github\.io\/ia-usage-bar\/<\/loc>/);
  assert.match(sitemap, /lang=en/);
  assert.match(sitemap, /lang=es/);
});

test("default document is English SEO with crawlable language alternates", () => {
  assert.match(html, /<html lang="en">/);
  assert.match(html, /<title>IA Usage – Claude, ChatGPT, Codex &amp; Cursor Usage Monitor<\/title>/);
  assert.match(html, /hreflang="en"/);
  assert.match(html, /hreflang="es"/);
  assert.match(html, /hreflang="x-default"/);
  assert.match(html, /Monitor Claude, ChatGPT, Codex, Cursor and AI usage limits/);
  assert.match(html, /Todos tus límites de IA/);
});

test("JSON-LD describes a free SoftwareApplication", () => {
  assert.match(html, /"@type": "SoftwareApplication"/);
  assert.match(html, /"price": "0"/);
  assert.match(html, /Claude Code/);
});

test("query lang wins over saved and browser language", () => {
  assert.equal(resolveLang({ search: "?lang=es", saved: "en", browser: "en-US" }), "es");
  assert.equal(resolveLang({ search: "?lang=en", saved: "es", browser: "es-ES" }), "en");
  assert.equal(resolveLang({ search: "", saved: "es", browser: "en-US" }), "es");
  assert.equal(resolveLang({ search: "", saved: null, browser: "es-VE" }), "es");
  assert.equal(resolveLang({ search: "", saved: null, browser: "en-US" }), "en");
});
