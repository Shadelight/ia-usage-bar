// Confirms Open VSX caught up to package.json after `ovsx publish`.
// Open VSX can lag a few seconds, so this retries instead of failing once.
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const pkg = JSON.parse(readFileSync(join(root, "package.json"), "utf8"));
const url = `https://open-vsx.org/api/${pkg.publisher}/${pkg.name}`;
const attempts = 12;
const delayMs = 5_000;

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

async function publishedVersion() {
  const response = await fetch(url, { headers: { Accept: "application/json" } });
  if (response.status === 404) return null;
  if (!response.ok) throw new Error(`Open VSX responded ${response.status} for ${url}`);
  const data = await response.json();
  return data.version ?? null;
}

let last = null;
for (let i = 1; i <= attempts; i++) {
  last = await publishedVersion();
  if (last === pkg.version) {
    console.log(`verify-openvsx: ${url} is ${last}`);
    process.exit(0);
  }
  console.log(`verify-openvsx: attempt ${i}/${attempts} saw ${last ?? "(none)"}, expected ${pkg.version}`);
  if (i < attempts) await sleep(delayMs);
}

console.error(
  `verify-openvsx: Open VSX still has ${last ?? "(none)"}, expected ${pkg.version}. ` +
    `The VSIX was packaged but the registry did not update.`,
);
process.exit(1);
