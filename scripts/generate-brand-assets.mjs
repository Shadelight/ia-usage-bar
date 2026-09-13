import { spawnSync } from "node:child_process";
import { copyFile, mkdir, readdir, readFile, rm, writeFile } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import process from "node:process";

const root = resolve(import.meta.dirname, "..");
const source = join(root, "assets", "brand", "source");
const generated = join(root, "assets", "brand", "generated");
const work = join(generated, ".work");
const tauri = join(root, "node_modules", "@tauri-apps", "cli", "tauri.js");

const masters = {
  mark: join(source, "ia-usage-mark.svg"),
  small: join(source, "ia-usage-small-mark.svg"),
  mono: join(source, "ia-usage-mono.svg"),
  app: join(source, "ia-usage-app-icon.svg"),
  wordmarkLight: join(source, "ia-usage-wordmark-light.svg"),
  wordmarkDark: join(source, "ia-usage-wordmark-dark.svg"),
};

const concepts = {
  directions: join(root, "assets", "brand", "concepts", "ia-usage-directions.svg"),
  applications: join(root, "assets", "brand", "concepts", "ia-usage-applications.svg"),
};

const palette = {
  background: "#18181B",
  surface: "#202023",
  primary: "#FF7043",
  primaryHover: "#FF835F",
  text: "#F5F5F5",
  muted: "#71717A",
  data: "#34D399",
};

async function ensureFile(path) {
  await readFile(path);
}

async function copy(sourcePath, targetPath) {
  await mkdir(dirname(targetPath), { recursive: true });
  await copyFile(sourcePath, targetPath);
}

function run(command, args) {
  const result = spawnSync(command, args, { cwd: root, encoding: "utf8", stdio: "pipe", windowsHide: true });
  if (result.status !== 0) {
    throw new Error(`${command} ${args.join(" ")} failed\n${result.error ?? ""}\n${result.stdout ?? ""}\n${result.stderr ?? ""}`);
  }
}

function tauriIcon(input, output, sizes = []) {
  const args = ["icon", input, "-o", output];
  for (const size of sizes) args.push("-p", String(size));
  run(process.execPath, [tauri, ...args]);
}

function packIco(images) {
  const headerSize = 6 + images.length * 16;
  const header = Buffer.alloc(headerSize);
  header.writeUInt16LE(0, 0);
  header.writeUInt16LE(1, 2);
  header.writeUInt16LE(images.length, 4);
  let offset = headerSize;
  images.forEach(({ size, bytes }, index) => {
    const entry = 6 + index * 16;
    header[entry] = size === 256 ? 0 : size;
    header[entry + 1] = size === 256 ? 0 : size;
    header[entry + 2] = 0;
    header[entry + 3] = 0;
    header.writeUInt16LE(1, entry + 4);
    header.writeUInt16LE(32, entry + 6);
    header.writeUInt32LE(bytes.length, entry + 8);
    header.writeUInt32LE(offset, entry + 12);
    offset += bytes.length;
  });
  return Buffer.concat([header, ...images.map(({ bytes }) => bytes)]);
}

function cropWithSystemDrawing(input, output, width, height, format) {
  if (process.platform !== "win32") {
    throw new Error("Brand generation currently requires Windows for deterministic PNG/BMP cropping.");
  }
  const escapedInput = input.replaceAll("'", "''");
  const escapedOutput = output.replaceAll("'", "''");
  const script = [
    "Add-Type -AssemblyName System.Drawing",
    `$src=[System.Drawing.Image]::FromFile('${escapedInput}')`,
    `$dst=New-Object System.Drawing.Bitmap(${width},${height})`,
    "$g=[System.Drawing.Graphics]::FromImage($dst)",
    "$g.DrawImage($src,0,0,$src.Width,$src.Height)",
    `$dst.Save('${escapedOutput}',[System.Drawing.Imaging.ImageFormat]::${format})`,
    "$g.Dispose()",
    "$dst.Dispose()",
    "$src.Dispose()",
  ].join("; ");
  run("powershell.exe", ["-NoProfile", "-Command", script]);
}

function androidVector({ monochrome = false } = {}) {
  const paths = monochrome
    ? `<path android:fillColor="${palette.text}" android:pathData="M30,50H62C65.3,50 68,52.7 68,56V202C68,205.3 65.3,208 62,208H36C32.7,208 30,205.3 30,202Z" />
    <path android:fillColor="${palette.text}" android:pathData="M82,204L143,55C146,47 152,43 160,43C168,43 174,47 178,55L246,204H202L159,98L124,204Z" />
    <path android:fillColor="${palette.text}" android:pathData="M133,151H174C175.7,151 177,152.3 177,154V166C177,167.7 175.7,169 174,169H133C131.3,169 130,167.7 130,166V154C130,152.3 131.3,151 133,151Z" />`
    : `<path android:fillColor="${palette.primary}" android:pathData="M33,48H53C56.9,48 60,51.1 60,55V201C60,204.9 56.9,208 53,208H33C29.1,208 26,204.9 26,201V55C26,51.1 29.1,48 33,48Z" />
    <path android:fillColor="${palette.text}" android:pathData="M82,204L142,55C145,47 151,43 159,43C167,43 173,47 177,55L246,204H204L157,96L121,204Z" />
    <path android:fillColor="${palette.primary}" android:pathData="M119,171H128C129.7,171 131,172.3 131,174V201C131,202.7 129.7,204 128,204H119C117.3,204 116,202.7 116,201V174C116,172.3 117.3,171 119,171ZM142,145H151C152.7,145 154,146.3 154,148V201C154,202.7 152.7,204 151,204H142C140.3,204 139,202.7 139,201V148C139,146.3 140.3,145 142,145ZM165,119H174C175.7,119 177,120.3 177,122V201C177,202.7 175.7,204 174,204H165C163.3,204 162,202.7 162,201V122C162,120.3 163.3,119 165,119Z" />`;
  return `<?xml version="1.0" encoding="utf-8"?>
<vector xmlns:android="http://schemas.android.com/apk/res/android"
    android:width="108dp" android:height="108dp"
    android:viewportWidth="256" android:viewportHeight="256">
    ${paths}
</vector>
`;
}

async function copyAndroidTree(from, to) {
  for (const entry of await readdir(from, { withFileTypes: true })) {
    if (!entry.isDirectory() || !entry.name.startsWith("mipmap-")) continue;
    const targetDir = join(to, entry.name);
    await mkdir(targetDir, { recursive: true });
    for (const file of await readdir(join(from, entry.name), { withFileTypes: true })) {
      if (file.isFile()) await copy(join(from, entry.name, file.name), join(targetDir, file.name));
    }
  }
}

const markGeometryDark = `
  <rect x="26" y="48" width="34" height="160" rx="7" fill="${palette.primary}"/>
  <path d="M82 204 142 55c3-8 9-12 17-12s14 4 18 12l69 149h-42L157 96l-36 108Z" fill="${palette.text}"/>
  <g fill="${palette.primary}"><rect x="116" y="171" width="15" height="33" rx="3"/><rect x="139" y="145" width="15" height="59" rx="3"/><rect x="162" y="119" width="15" height="85" rx="3"/></g>`;

const markGeometryLight = markGeometryDark.replace(`fill="${palette.text}"`, `fill="${palette.background}"`);

const markGeometryMono = `
  <rect x="30" y="50" width="38" height="158" rx="6" fill="${palette.text}"/>
  <path d="M82 204 143 55c3-8 9-12 17-12s14 4 18 12l68 149h-44L159 98l-35 106Z" fill="${palette.text}"/>
  <rect x="130" y="151" width="47" height="18" rx="3" fill="${palette.text}"/>`;

const smallGeometryDark = `
  <rect x="30" y="50" width="38" height="158" rx="6" fill="${palette.primary}"/>
  <path d="M82 204 143 55c3-8 9-12 17-12s14 4 18 12l68 149h-44L159 98l-35 106Z" fill="${palette.text}"/>
  <rect x="130" y="151" width="47" height="18" rx="3" fill="${palette.primary}"/>`;

const smallGeometryLight = smallGeometryDark.replace(`fill="${palette.text}"`, `fill="${palette.background}"`);

function smallBadgeSvg(monochrome = false) {
  const geometry = monochrome ? markGeometryMono : smallGeometryDark;
  return canvasSvg(256, `<rect x="8" y="8" width="240" height="240" rx="52" fill="${palette.background}"/><rect x="8.5" y="8.5" width="239" height="239" rx="51.5" fill="none" stroke="#343439"/><g transform="translate(18 18) scale(.86)">${geometry}</g>`);
}

function canvasSvg(width, content) {
  return `<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${width}" viewBox="0 0 ${width} ${width}">${content}</svg>`;
}

async function main() {
  await Promise.all([...Object.values(masters), ...Object.values(concepts)].map(ensureFile));
  await rm(generated, { recursive: true, force: true });
  await mkdir(work, { recursive: true });

  const appSizes = [16, 20, 24, 32, 40, 48, 64, 72, 96, 128, 144, 180, 192, 256, 512];
  const icoSizes = [16, 20, 24, 32, 40, 48, 64, 128, 256];
  const appPng = join(work, "app-png");
  tauriIcon(masters.app, appPng, appSizes);

  const favicon32Svg = join(work, "favicon-small.svg");
  const favicon16Svg = join(work, "favicon-tiny-mono.svg");
  await writeFile(favicon32Svg, smallBadgeSvg());
  await writeFile(favicon16Svg, smallBadgeSvg(true));
  const favicon32Png = join(work, "favicon-small-png");
  const favicon16Png = join(work, "favicon-tiny-png");
  tauriIcon(favicon32Svg, favicon32Png, [32]);
  tauriIcon(favicon16Svg, favicon16Png, [16]);

  const icoImages = await Promise.all(icoSizes.map(async (size) => ({ size, bytes: await readFile(join(appPng, `${size}x${size}.png`)) })));
  const appIco = packIco(icoImages);

  const windows = join(generated, "windows");
  await mkdir(windows, { recursive: true });
  for (const size of icoSizes) await copy(join(appPng, `${size}x${size}.png`), join(windows, `${size}x${size}.png`));
  await writeFile(join(windows, "icon.ico"), appIco);

  const androidWork = join(work, "android");
  tauriIcon(masters.app, androidWork);
  const generatedAndroidRes = join(generated, "android", "res");
  await copyAndroidTree(join(androidWork, "android"), generatedAndroidRes);
  await mkdir(join(generatedAndroidRes, "drawable"), { recursive: true });
  await mkdir(join(generatedAndroidRes, "mipmap-anydpi-v26"), { recursive: true });
  await mkdir(join(generatedAndroidRes, "mipmap-anydpi-v33"), { recursive: true });
  await mkdir(join(generatedAndroidRes, "values"), { recursive: true });
  await writeFile(join(generatedAndroidRes, "drawable", "ic_ia_usage_mark.xml"), androidVector());
  await writeFile(join(generatedAndroidRes, "drawable", "ic_ia_usage_mark_mono.xml"), androidVector({ monochrome: true }));
  await writeFile(join(generatedAndroidRes, "drawable", "ic_ia_usage_mark_foreground.xml"), `<?xml version="1.0" encoding="utf-8"?>\n<inset xmlns:android="http://schemas.android.com/apk/res/android" android:drawable="@drawable/ic_ia_usage_mark" android:inset="18%" />\n`);
  await writeFile(join(generatedAndroidRes, "drawable", "ic_ia_usage_mono_foreground.xml"), `<?xml version="1.0" encoding="utf-8"?>\n<inset xmlns:android="http://schemas.android.com/apk/res/android" android:drawable="@drawable/ic_ia_usage_mark_mono" android:inset="18%" />\n`);
  const adaptive26 = `<?xml version="1.0" encoding="utf-8"?>\n<adaptive-icon xmlns:android="http://schemas.android.com/apk/res/android">\n    <background android:drawable="@color/ia_usage_background" />\n    <foreground android:drawable="@drawable/ic_ia_usage_mark_foreground" />\n</adaptive-icon>\n`;
  const adaptive33 = `<?xml version="1.0" encoding="utf-8"?>\n<adaptive-icon xmlns:android="http://schemas.android.com/apk/res/android">\n    <background android:drawable="@color/ia_usage_background" />\n    <foreground android:drawable="@drawable/ic_ia_usage_mark_foreground" />\n    <monochrome android:drawable="@drawable/ic_ia_usage_mono_foreground" />\n</adaptive-icon>\n`;
  for (const name of ["ic_launcher.xml", "ic_launcher_round.xml"]) {
    await writeFile(join(generatedAndroidRes, "mipmap-anydpi-v26", name), adaptive26);
    await writeFile(join(generatedAndroidRes, "mipmap-anydpi-v33", name), adaptive33);
  }
  await writeFile(join(generatedAndroidRes, "values", "brand_colors.xml"), `<?xml version="1.0" encoding="utf-8"?>\n<resources>\n    <color name="ia_usage_background">${palette.background}</color>\n    <color name="ia_usage_surface">${palette.surface}</color>\n    <color name="ia_usage_primary">${palette.primary}</color>\n    <color name="ia_usage_text">${palette.text}</color>\n    <color name="ia_usage_muted">${palette.muted}</color>\n    <color name="ia_usage_data">${palette.data}</color>\n</resources>\n`);

  const vscode = join(generated, "vscode");
  await copy(join(appPng, "256x256.png"), join(vscode, "icon.png"));

  const web = join(generated, "web");
  await copy(favicon32Svg, join(web, "favicon.svg"));
  await copy(join(favicon16Png, "16x16.png"), join(web, "favicon-16.png"));
  await copy(join(favicon32Png, "32x32.png"), join(web, "favicon-32.png"));
  await copy(join(appPng, "48x48.png"), join(web, "favicon-48.png"));
  const faviconImages = [
    { size: 16, bytes: await readFile(join(favicon16Png, "16x16.png")) },
    { size: 32, bytes: await readFile(join(favicon32Png, "32x32.png")) },
    { size: 48, bytes: await readFile(join(appPng, "48x48.png")) },
  ];
  await writeFile(join(web, "favicon.ico"), packIco(faviconImages));
  await copy(join(appPng, "180x180.png"), join(web, "apple-touch-icon.png"));
  await copy(join(appPng, "192x192.png"), join(web, "icon-192.png"));
  await copy(join(appPng, "512x512.png"), join(web, "icon-512.png"));
  await copy(masters.wordmarkLight, join(web, "wordmark-light.svg"));
  await copy(masters.wordmarkDark, join(web, "wordmark-dark.svg"));
  await copy(masters.small, join(web, "small-mark.svg"));

  const ogSvg = canvasSvg(1200, `<rect width="1200" height="630" rx="40" fill="${palette.background}"/><rect x="54" y="54" width="1092" height="522" rx="30" fill="${palette.surface}" stroke="#343439" stroke-width="2"/><rect x="92" y="92" width="330" height="446" rx="24" fill="#1B1B1E"/><g transform="translate(120 168) scale(1.08)">${markGeometryDark}</g><text x="492" y="270" fill="${palette.text}" font-family="Segoe UI,Arial,sans-serif" font-size="96" font-weight="700" letter-spacing="-4">IA Usage</text><text x="498" y="328" fill="${palette.muted}" font-family="Segoe UI,Arial,sans-serif" font-size="24" letter-spacing="3">QUOTAS · LIMITS · RESETS</text><path d="M498 371H1068" stroke="${palette.primary}" stroke-width="5"/><text x="498" y="430" fill="#A1A1AA" font-family="Segoe UI,Arial,sans-serif" font-size="27">Windows · Android · VS Code</text><circle cx="510" cy="482" r="7" fill="${palette.data}"/><text x="531" y="491" fill="${palette.muted}" font-family="Segoe UI,Arial,sans-serif" font-size="20">usage data connected</text>`);
  const ogCanvas = join(work, "og-canvas.svg");
  await writeFile(ogCanvas, ogSvg);
  const ogRaster = join(work, "og-raster");
  tauriIcon(ogCanvas, ogRaster, [1200]);
  cropWithSystemDrawing(join(ogRaster, "1200x1200.png"), join(web, "og-image.png"), 1200, 630, "Png");

  const headerSvg = canvasSvg(150, `<rect width="150" height="57" fill="${palette.background}"/><rect x="0" y="55" width="150" height="2" fill="${palette.primary}"/><g transform="translate(5 7) scale(.17)">${markGeometryDark}</g><text x="52" y="35" fill="${palette.text}" font-family="Segoe UI,Arial,sans-serif" font-size="17" font-weight="700">IA Usage</text>`);
  const headerCanvas = join(work, "header-canvas.svg");
  await writeFile(headerCanvas, headerSvg);
  const headerRaster = join(work, "header-raster");
  tauriIcon(headerCanvas, headerRaster, [150]);
  cropWithSystemDrawing(join(headerRaster, "150x150.png"), join(windows, "nsis-header.bmp"), 150, 57, "Bmp");

  const sizes = [16, 20, 24, 32, 48, 64, 128, 256];
  const rows = [
    { y: 175, bg: palette.text, label: "LIGHT SURFACE", text: palette.background, geometry: markGeometryLight },
    { y: 500, bg: palette.surface, label: "DARK SURFACE", text: palette.text, geometry: markGeometryDark },
    { y: 825, bg: palette.background, label: "MONO / SYSTEM", text: palette.text, geometry: markGeometryMono },
  ];
  const sheetItems = rows.map((row) => {
    let x = 150;
    const marks = sizes.map((size) => {
      const scale = size / 256;
      const geometry = size <= 32
        ? row.label === "LIGHT SURFACE" ? smallGeometryLight : row.label === "DARK SURFACE" ? smallGeometryDark : markGeometryMono
        : row.geometry;
      const item = `<g transform="translate(${x} ${row.y + 62 - size / 2}) scale(${scale})">${geometry}</g><text x="${x + size / 2}" y="${row.y + 150}" text-anchor="middle" fill="${row.text}" font-family="Segoe UI,Arial,sans-serif" font-size="17">${size}</text>`;
      x += Math.max(size + 42, 92);
      return item;
    }).join("");
    return `<rect x="56" y="${row.y - 55}" width="1088" height="250" rx="28" fill="${row.bg}"/><text x="92" y="${row.y - 13}" fill="${row.text}" font-family="Segoe UI,Arial,sans-serif" font-size="18" font-weight="700" letter-spacing="3">${row.label}</text>${marks}`;
  }).join("");
  const sheetSvg = canvasSvg(1200, `<rect width="1200" height="1200" fill="#D4D4D8"/><text x="56" y="72" fill="${palette.background}" font-family="Segoe UI,Arial,sans-serif" font-size="42" font-weight="750">IA Usage · legibility sheet</text><text x="56" y="105" fill="${palette.muted}" font-family="Segoe UI,Arial,sans-serif" font-size="18">Official small-size system · actual pixel sizes</text>${sheetItems}`);
  const sheetCanvas = join(work, "legibility-sheet.svg");
  await writeFile(sheetCanvas, sheetSvg);
  const sheetRaster = join(work, "sheet-raster");
  tauriIcon(sheetCanvas, sheetRaster, [1200]);
  await copy(join(sheetRaster, "1200x1200.png"), join(generated, "brand-legibility-sheet.png"));

  for (const [name, input] of Object.entries(concepts)) {
    const conceptRaster = join(work, `concept-${name}`);
    tauriIcon(input, conceptRaster, [1400]);
    await copy(join(conceptRaster, "1400x1400.png"), join(generated, `brand-${name}.png`));
  }

  await copy(join(windows, "32x32.png"), join(root, "src-tauri", "icons", "32x32.png"));
  await copy(join(windows, "128x128.png"), join(root, "src-tauri", "icons", "128x128.png"));
  await copy(join(windows, "256x256.png"), join(root, "src-tauri", "icons", "128x128@2x.png"));
  await copy(join(windows, "icon.ico"), join(root, "src-tauri", "icons", "icon.ico"));
  await copy(join(windows, "nsis-header.bmp"), join(root, "src-tauri", "windows", "header.bmp"));

  const androidRes = join(root, "android", "app", "src", "main", "res");
  await copyAndroidTree(generatedAndroidRes, androidRes);
  for (const directory of ["drawable", "values"]) {
    for (const file of await readdir(join(generatedAndroidRes, directory))) {
      await copy(join(generatedAndroidRes, directory, file), join(androidRes, directory, file));
    }
  }

  await copy(join(vscode, "icon.png"), join(root, "apps", "vscode-extension", "images", "icon.png"));

  const website = join(root, "website");
  for (const file of await readdir(web)) await copy(join(web, file), join(website, file));
  await writeFile(join(website, "site.webmanifest"), JSON.stringify({
    name: "IA Usage",
    short_name: "IA Usage",
    description: "AI usage limits across Windows, Android and VS Code.",
    start_url: "./",
    display: "standalone",
    background_color: palette.background,
    theme_color: palette.background,
    icons: [
      { src: "./icon-192.png", sizes: "192x192", type: "image/png" },
      { src: "./icon-512.png", sizes: "512x512", type: "image/png" },
      { src: "./icon-512.png", sizes: "512x512", type: "image/png", purpose: "maskable" },
    ],
  }, null, 2) + "\n");

  await copy(masters.small, join(root, "src", "assets", "brand", "ia-usage-small-mark.svg"));
  await copy(masters.wordmarkLight, join(root, "src", "assets", "brand", "ia-usage-wordmark-light.svg"));
  await copy(masters.wordmarkDark, join(root, "src", "assets", "brand", "ia-usage-wordmark-dark.svg"));

  await rm(work, { recursive: true, force: true });
  console.log("IA Usage brand assets generated and wired to Windows, Android, VS Code and Web.");
}

await main();
