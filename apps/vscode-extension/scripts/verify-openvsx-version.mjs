// Confirms this package.json version exists on Open VSX after `ovsx publish`.
// Do not require `latest` to have flipped: the alias can lag a minute behind
// the versioned document, which is what actually proves the upload landed.
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const pkg = JSON.parse(readFileSync(join(root, "package.json"), "utf8"));
const url = `https://open-vsx.org/api/${pkg.publisher}/${pkg.name}`;
const attempts = 18;
const delayMs = 5_000;

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

async function fetchJson(target) {
  const response = await fetch(target, { headers: { Accept: "application/json" } });
  if (response.status === 404) return null;
  if (!response.ok) throw new Error(`Open VSX responded ${response.status} for ${target}`);
  return response.json();
}

function hasVersion(data, version) {
  if (!data) return false;
  if (data.version === version) return true;
  return Boolean(data.allVersions && data.allVersions[version]);
}

async function versionIsLive(version) {
  const latest = await fetchJson(url);
  if (hasVersion(latest, version)) return true;
  const specific = await fetchJson(`${url}/${version}`);
  return hasVersion(specific, version);
}

let last = null;
for (let i = 1; i <= attempts; i++) {
  try {
    if (await versionIsLive(pkg.version)) {
      console.log(`verify-openvsx: ${url}/${pkg.version} is live`);
      process.exit(0);
    }
    const latest = await fetchJson(url);
    last = latest?.version ?? null;
  } catch (error) {
    last = String(error.message);
  }
  console.log(`verify-openvsx: attempt ${i}/${attempts} latest=${last ?? "(none)"}, looking for ${pkg.version}`);
  if (i < attempts) await sleep(delayMs);
}

console.error(
  `verify-openvsx: ${pkg.version} is not visible at ${url} (latest still ${last ?? "unknown"}).`,
);
process.exit(1);
