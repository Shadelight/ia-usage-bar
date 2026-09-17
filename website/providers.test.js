import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

import { PROVIDERS } from "./providers.js";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");

function parseRustMatch(src, fnName) {
  const header = new RegExp(`fn ${fnName}\\(self\\) -> &'static str \\{\\s*match self \\{`);
  const start = src.search(header);
  assert.ok(start >= 0, `missing ${fnName}`);
  const bodyStart = src.indexOf("{", src.indexOf("match self", start)) + 1;
  let depth = 1;
  let i = bodyStart;
  while (i < src.length && depth > 0) {
    if (src[i] === "{") depth += 1;
    else if (src[i] === "}") depth -= 1;
    i += 1;
  }
  const body = src.slice(bodyStart, i - 1);
  /** @type {Record<string, string>} */
  const out = {};
  const arm = /VendorId::(\w+)\s*=>\s*"([^"]+)"/g;
  let match;
  while ((match = arm.exec(body))) {
    out[match[1]] = match[2];
  }
  return out;
}

test("landing catalog covers every VendorId display name", () => {
  const src = readFileSync(join(root, "crates/iausage-core/src/model.rs"), "utf8");
  const slugs = parseRustMatch(src, "slug");
  const names = parseRustMatch(src, "display_name");
  const byId = Object.fromEntries(PROVIDERS.map((provider) => [provider.id, provider]));

  for (const [variant, slug] of Object.entries(slugs)) {
    assert.ok(byId[slug], `missing landing chip for ${variant} (${slug})`);
    assert.equal(byId[slug].name, names[variant], `display name drift for ${slug}`);
  }

  assert.equal(PROVIDERS.length, Object.keys(slugs).length);
});

test("provider icons exist on disk or use the monogram fallback", () => {
  for (const provider of PROVIDERS) {
    if (!provider.icon) {
      assert.equal(provider.id, "commandcode");
      continue;
    }
    const path = join(root, "website/assets/providers", provider.icon);
    readFileSync(path);
  }
});
