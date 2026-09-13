// Fails the build if package.json's version is not strictly greater than
// what's already published on Open VSX — this is exactly how 0.2.7 briefly
// regressed behind an already-published, already-installed 0.3.0.
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const pkg = JSON.parse(readFileSync(join(root, "package.json"), "utf8"));
const [namespace, name] = [pkg.publisher, pkg.name];

function parseSemver(version) {
  const [core] = version.split("-");
  const parts = core.split(".").map((n) => Number.parseInt(n, 10));
  if (parts.length !== 3 || parts.some(Number.isNaN)) throw new Error(`Versión no semver: ${version}`);
  return parts;
}

function isGreater(a, b) {
  const pa = parseSemver(a);
  const pb = parseSemver(b);
  for (let i = 0; i < 3; i++) {
    if (pa[i] !== pb[i]) return pa[i] > pb[i];
  }
  return false;
}

async function publishedVersion() {
  const url = `https://open-vsx.org/api/${namespace}/${name}`;
  const response = await fetch(url);
  if (response.status === 404) return null; // never published: any version is fine
  if (!response.ok) throw new Error(`Open VSX respondió ${response.status} consultando ${url}`);
  const data = await response.json();
  return data.version ?? null;
}

const published = await publishedVersion().catch((error) => {
  console.warn(`check-published-version: no se pudo consultar Open VSX (${error.message}); se omite el chequeo.`);
  return undefined;
});

if (published === undefined) process.exit(0); // network/API failure: don't block local packaging on it
if (published !== null && !isGreater(pkg.version, published)) {
  console.error(
    `check-published-version: package.json tiene ${pkg.version}, pero Open VSX ya publicó ${published}. ` +
      `Sube la versión antes de empaquetar/publicar.`,
  );
  process.exit(1);
}
console.log(`check-published-version: ${pkg.version} > ${published ?? "(sin publicar)"} — OK`);
