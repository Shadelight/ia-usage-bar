// Builds GitHub release notes from the matching CHANGELOG.md section.
//
// Usage:
//   node scripts/build-release-notes.mjs <version|tag> [--prev-tag vX.Y.Z] [--repo owner/name]
//
// Prints markdown to stdout. Exits non-zero when no `## [<version>]`
// section exists, so the release workflow fails fast instead of
// publishing a build with empty notes.
import { readFileSync } from "node:fs";
import { execSync } from "node:child_process";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const SECTION_HEADINGS_ES = {
  added: "Novedades",
  changed: "Cambios",
  deprecated: "Obsoleto",
  removed: "Eliminado",
  fixed: "Correcciones",
  security: "Seguridad",
};

function parseArgs(argv) {
  const out = { version: null, prevTag: "", repo: "" };
  const rest = [...argv];
  while (rest.length > 0) {
    const arg = rest.shift();
    if (arg === "--prev-tag") out.prevTag = rest.shift() ?? "";
    else if (arg === "--repo") out.repo = rest.shift() ?? "";
    else if (!out.version) out.version = arg;
    else throw new Error(`Argumento inesperado: ${arg}`);
  }
  if (!out.version) throw new Error("Falta la versión: build-release-notes.mjs <version|tag>");
  out.version = out.version.replace(/^v/, "");
  return out;
}

function repoFromOrigin(root) {
  try {
    const url = execSync("git config --get remote.origin.url", { cwd: root, encoding: "utf8" }).trim();
    const match = url.match(/github\.com[/:]([^/]+\/[^/]+?)(?:\.git)?$/);
    return match ? match[1] : "";
  } catch {
    return "";
  }
}

function extractSection(changelog, version) {
  const text = changelog.replace(/\r\n/g, "\n");
  const header = new RegExp(`^## \\[${version.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}\\].*$`, "m");
  const start = text.match(header);
  if (!start || start.index === undefined) return null;
  const bodyStart = start.index + start[0].length;
  const nextSection = text.slice(bodyStart).search(/^## /m);
  let body = (nextSection === -1 ? text.slice(bodyStart) : text.slice(bodyStart, bodyStart + nextSection)).trim();
  // Las definiciones de links del CHANGELOG ([etiqueta]: url) viven al final
  // del archivo y no pertenecen a las notas del release.
  body = body
    .split("\n")
    .filter((line) => !/^\[[^\]]+\]:\s*\S/.test(line))
    .join("\n")
    .trim();
  return body;
}

function renderNotes({ preamble, groups, tag, prevTag, repo }) {
  const lines = [];
  if (preamble) lines.push(preamble, "");
  for (const [heading, body] of groups) {
    lines.push(`## ${heading}`, "", body, "");
  }
  lines.push(
    "## Instalación",
    "",
    "Descarga el instalador `.exe` o el paquete `.msi` desde **Assets** y ejecútalo. " +
      "Para verificar la descarga, compara su hash con el publicado en `SHA256SUMS.txt`:",
    "",
    "```powershell",
    "(Get-FileHash .\\instalador.exe -Algorithm SHA256).Hash",
    "```",
    "",
  );
  if (repo && prevTag) {
    lines.push(`**Historial completo**: https://github.com/${repo}/compare/${prevTag}...${tag}`, "");
  } else if (repo) {
    lines.push(`**Historial completo**: https://github.com/${repo}/commits/${tag}`, "");
  }
  return `${lines.join("\n").trim()}\n`;
}

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const { version, prevTag, repo: repoFlag } = parseArgs(process.argv.slice(2));
const repo = repoFlag || repoFromOrigin(root);

const changelog = readFileSync(join(root, "CHANGELOG.md"), "utf8");
const section = extractSection(changelog, version);
if (!section) {
  console.error(`build-release-notes: CHANGELOG.md no tiene sección "## [${version}]"`);
  process.exit(1);
}

const parts = section.split(/^### /m);
const preamble = (parts.shift() ?? "").trim();
const groups = [];
for (const part of parts) {
  const newline = part.indexOf("\n");
  const name = (newline === -1 ? part : part.slice(0, newline)).trim().toLowerCase();
  const body = (newline === -1 ? "" : part.slice(newline + 1)).trim();
  if (!body) continue;
  groups.push([SECTION_HEADINGS_ES[name] ?? name, body]);
}

process.stdout.write(renderNotes({ preamble, groups, tag: `v${version}`, prevTag, repo }));
