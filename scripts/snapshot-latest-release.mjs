// Writes website/latest-release.json from GitHub's latest published release.
// Used by the Pages workflow so first paint has download URLs without a
// client round-trip. Safe to run locally: `node scripts/snapshot-latest-release.mjs`
import { writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { GITHUB_API_LATEST, parseGithubRelease } from "../website/releases.js";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const outPath = join(root, "website", "latest-release.json");
const token = process.env.GITHUB_TOKEN || process.env.GH_TOKEN || "";

const headers = {
  Accept: "application/vnd.github+json",
  "User-Agent": "IA-Usage-Landing",
};
if (token) headers.Authorization = `Bearer ${token}`;

const response = await fetch(GITHUB_API_LATEST, { headers });
if (!response.ok) {
  console.error(`snapshot-latest-release: GitHub API HTTP ${response.status}`);
  process.exit(1);
}

const release = parseGithubRelease(await response.json());
if (!release.ok) {
  console.error("snapshot-latest-release: could not parse latest release");
  process.exit(1);
}

writeFileSync(outPath, `${JSON.stringify(release, null, 2)}\n`);
console.log(`wrote ${outPath} (${release.tag})`);
