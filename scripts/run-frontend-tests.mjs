// Runs the TypeScript frontend tests on any Node.js version.
//
// `node --test tests/*.test.ts` relies on native type stripping, which only
// exists on Node 22.6+/23.6+ (enabled by default on Node 24). On older
// runtimes it fails with ERR_UNKNOWN_FILE_EXTENSION. This runner transpiles
// the test closure to plain JavaScript with the repo's existing `typescript`
// devDependency and then executes the emitted files with `node --test`,
// so the suite behaves identically everywhere.
import { spawnSync } from "node:child_process";
import { copyFileSync, existsSync, mkdirSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import ts from "typescript";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const testsDir = join(root, "tests");
const outDir = join(root, "node_modules", ".cache", "iausagebar-frontend-tests");

// Relative module specifiers inside import/export-from and dynamic import().
const SPECIFIER_RE = /(from\s+["']|import\s*\(\s*["']|import\s+["'])(\.[^"']+)(["']\s*\)?)/g;
// Static asset reads such as new URL("../src/styles.css", import.meta.url)
// resolve against the importing file, so mirror the referenced file next to
// the emitted output to keep them working.
const ASSET_URL_RE = /new\s+URL\(\s*["'](\.[^"']+)["']\s*,\s*import\.meta\.url\s*\)/g;

function rewriteTsExtensions(code) {
  return code.replace(SPECIFIER_RE, (match, open, spec, close) =>
    spec.endsWith(".ts") ? `${open}${spec.slice(0, -3)}.js${close}` : match,
  );
}

function collectRelativeTsImports(source) {
  const specs = new Set();
  for (const match of source.matchAll(SPECIFIER_RE)) {
    const spec = match[2];
    if (spec.endsWith(".ts")) specs.add(spec);
  }
  return specs;
}

function transpile(tsPath) {
  const source = readFileSync(tsPath, "utf8");
  const { outputText } = ts.transpileModule(source, {
    compilerOptions: {
      module: ts.ModuleKind.ESNext,
      target: ts.ScriptTarget.ES2020,
    },
    fileName: tsPath,
  });
  return rewriteTsExtensions(outputText);
}

// Follow relative `.ts` imports starting from each test entry so the emitted
// tree mirrors the repo layout and relative imports keep resolving.
function collectClosure(entries) {
  const seen = new Set();
  const queue = [...entries];
  while (queue.length > 0) {
    const current = queue.pop();
    if (seen.has(current)) continue;
    seen.add(current);
    const source = readFileSync(current, "utf8");
    for (const spec of collectRelativeTsImports(source)) {
      const resolved = resolve(dirname(current), spec);
      if (resolved.startsWith(root) && resolved.endsWith(".ts")) queue.push(resolved);
    }
  }
  return seen;
}

const onlyArgs = process.argv.slice(2).filter((arg) => !arg.startsWith("-"));
const nodeTestFlags = process.argv.slice(2).filter((arg) => arg.startsWith("-"));

let entries;
if (onlyArgs.length > 0) {
  entries = onlyArgs.map((arg) => resolve(root, arg));
} else {
  entries = readdirSync(testsDir)
    .filter((name) => name.endsWith(".test.ts"))
    .map((name) => join(testsDir, name))
    .sort();
}

if (entries.length === 0) {
  console.error("run-frontend-tests: no test entries found under tests/");
  process.exit(1);
}

const emittedTests = [];
const mirroredAssets = new Set();
for (const tsPath of collectClosure(entries)) {
  const outPath = join(outDir, `${relative(root, tsPath).replace(/\.ts$/, ".js")}`);
  mkdirSync(dirname(outPath), { recursive: true });
  const source = readFileSync(tsPath, "utf8");
  writeFileSync(outPath, transpile(tsPath));
  for (const match of source.matchAll(ASSET_URL_RE)) {
    const assetPath = resolve(dirname(tsPath), match[1]);
    if (assetPath.startsWith(root) && existsSync(assetPath)) {
      // .ts assets are mirrored verbatim (text reads like readFileSync of
      // sources); transpiled modules land as .js and never collide with them.
      mirroredAssets.add(assetPath);
    }
  }
  if (entries.includes(tsPath)) emittedTests.push(outPath);
}
for (const assetPath of mirroredAssets) {
  const outPath = join(outDir, relative(root, assetPath));
  mkdirSync(dirname(outPath), { recursive: true });
  copyFileSync(assetPath, outPath);
}

const result = spawnSync(process.execPath, ["--test", ...nodeTestFlags, ...emittedTests], {
  stdio: "inherit",
});
process.exit(result.status ?? 1);
