// Regenerates media/provider-icons/provider-icons.woff from the monochrome
// SVGs in media/font-src (copies of the same brand marks the desktop app
// uses under src/assets/providers, with fill="currentColor" so the status
// bar theme controls their color). The output .woff is checked into git
// like images/icon.png — this script only needs to run again when a
// provider icon is added/changed, so it's not wired into `npm run package`.
//
// fantasticon's own glob call breaks on Windows: it builds the search
// pattern with path.join(), which always emits backslashes on win32, and
// the `glob` package it delegates to treats backslash as an escape
// character — every path segment silently vanishes. Monkeypatch its glob
// import to normalize to forward slashes before matching.
import path from "node:path";
import { createRequire } from "node:module";
import { fileURLToPath } from "node:url";

// require(), not import(): fantasticon's ESM entrypoint pulls in a separate
// chunk that resolves its own `glob` instance, which the monkeypatch below
// never touches. Its CJS build (dist/index.cjs) shares the same `glob`
// module instance we patch here.
const require = createRequire(import.meta.url);
const globPkg = require("glob");
const originalGlob = globPkg.glob;
globPkg.glob = (pattern, opts) => originalGlob(String(pattern).split(path.sep).join("/"), opts);

const { generateFonts } = require("fantasticon");

const root = path.dirname(fileURLToPath(import.meta.url)) + "/..";
const result = await generateFonts({
  inputDir: path.join(root, "media/font-src"),
  outputDir: path.join(root, "media/provider-icons"),
  name: "provider-icons",
  fontTypes: ["woff"],
  assetTypes: ["json"],
  normalize: true,
});

console.log("Codepoints (paste the hex form into package.json's contributes.icons):");
for (const [id, codepoint] of Object.entries(result.codepoints)) {
  console.log(`  ${id}: \\${codepoint.toString(16).toUpperCase()}`);
}
