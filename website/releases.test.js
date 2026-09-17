import assert from "node:assert/strict";
import test from "node:test";

import {
  CACHE_KEY,
  FALLBACK_RELEASE_URL,
  downloadUrl,
  fallbackRelease,
  getLatestRelease,
  parseGithubRelease,
  resolveReleaseAssets,
} from "./releases.js";

const FIXTURE_ASSETS = [
  { name: "IA-Usage-Android-0.3.7.aab", browser_download_url: "https://example.test/app.aab" },
  { name: "IA-Usage-Android-0.3.7.apk", browser_download_url: "https://example.test/app.apk" },
  { name: "IA-Usage-VSCode-0.3.7.vsix", browser_download_url: "https://example.test/ext.vsix" },
  { name: "IA.Usage.Bar_0.3.7_x64-setup.exe", browser_download_url: "https://example.test/setup.exe" },
  { name: "IA.Usage.Bar_0.3.7_x64_en-US.msi", browser_download_url: "https://example.test/app.msi" },
  { name: "SHA256SUMS.txt", browser_download_url: "https://example.test/SHA256SUMS.txt" },
  { name: "source.zip", browser_download_url: "https://example.test/source.zip" },
  { name: "source.tar.gz", browser_download_url: "https://example.test/source.tar.gz" },
];

test("prefers the NSIS setup.exe over a generic exe", () => {
  const assets = [
    { name: "IA.Usage.Bar_0.3.7_x64.exe", browser_download_url: "https://example.test/plain.exe" },
    { name: "IA.Usage.Bar_0.3.7_x64-setup.exe", browser_download_url: "https://example.test/setup.exe" },
  ];
  assert.equal(resolveReleaseAssets(assets).windowsExe, "https://example.test/setup.exe");
});

test("selects the Windows MSI", () => {
  assert.equal(resolveReleaseAssets(FIXTURE_ASSETS).windowsMsi, "https://example.test/app.msi");
});

test("selects the Android APK and ignores the AAB", () => {
  const resolved = resolveReleaseAssets(FIXTURE_ASSETS);
  assert.equal(resolved.androidApk, "https://example.test/app.apk");
  assert.equal(resolved.windowsExe, "https://example.test/setup.exe");
  assert.doesNotMatch(JSON.stringify(resolved), /\.aab/);
});

test("selects the VS Code VSIX", () => {
  assert.equal(resolveReleaseAssets(FIXTURE_ASSETS).vscodeVsix, "https://example.test/ext.vsix");
});

test("ignores source.zip and source.tar.gz", () => {
  const resolved = resolveReleaseAssets(FIXTURE_ASSETS);
  const urls = Object.values(resolved);
  assert.ok(!urls.some((url) => /source\.(zip|tar\.gz)/.test(url)));
});

test("missing assets are omitted rather than pointing at the wrong file", () => {
  const resolved = resolveReleaseAssets([
    { name: "IA.Usage.Bar_0.3.7_x64-setup.exe", browser_download_url: "https://example.test/setup.exe" },
  ]);
  assert.equal(resolved.windowsExe, "https://example.test/setup.exe");
  assert.equal(resolved.windowsMsi, undefined);
  assert.equal(resolved.androidApk, undefined);
  assert.equal(resolved.vscodeVsix, undefined);
});

test("parseGithubRelease strips the v prefix and keeps the tag", () => {
  const release = parseGithubRelease({
    tag_name: "v0.3.7",
    published_at: "2026-09-14T22:12:07Z",
    html_url: "https://github.com/Shadelight/ia-usage-bar/releases/tag/v0.3.7",
    assets: FIXTURE_ASSETS,
  });
  assert.equal(release.ok, true);
  assert.equal(release.tag, "v0.3.7");
  assert.equal(release.version, "0.3.7");
  assert.equal(release.windowsExe, "https://example.test/setup.exe");
  assert.equal(release.checksums, "https://example.test/SHA256SUMS.txt");
});

test("downloadUrl never returns #, empty, or undefined", () => {
  const fallback = fallbackRelease();
  assert.equal(downloadUrl(fallback, "windowsExe"), FALLBACK_RELEASE_URL);
  assert.equal(downloadUrl(null, "androidApk"), FALLBACK_RELEASE_URL);
  assert.equal(downloadUrl(undefined, "vscodeVsix"), FALLBACK_RELEASE_URL);

  const partial = parseGithubRelease({
    tag_name: "v1.0.0",
    html_url: "https://github.com/Shadelight/ia-usage-bar/releases/tag/v1.0.0",
    assets: [{ name: "app.exe", browser_download_url: "https://example.test/app.exe" }],
  });
  assert.equal(downloadUrl(partial, "windowsExe"), "https://example.test/app.exe");
  assert.equal(downloadUrl(partial, "androidApk"), partial.htmlUrl);
  for (const key of ["windowsExe", "windowsMsi", "androidApk", "vscodeVsix"]) {
    const url = downloadUrl(partial, key);
    assert.ok(typeof url === "string" && url.startsWith("https://"));
    assert.notEqual(url, "#");
    assert.notEqual(url, "undefined");
  }
});

test("getLatestRelease falls back when GitHub fails", async () => {
  const storage = memoryStorage();
  const release = await getLatestRelease({
    fetchFn: async () => {
      throw new Error("network");
    },
    storage,
    skipSnapshot: true,
  });
  assert.equal(release.ok, false);
  assert.equal(release.version, "");
  assert.equal(release.htmlUrl, FALLBACK_RELEASE_URL);
  assert.equal(downloadUrl(release, "windowsExe"), FALLBACK_RELEASE_URL);
});

test("getLatestRelease uses a fresh localStorage cache", async () => {
  const storage = memoryStorage();
  const cached = {
    ok: true,
    tag: "v0.3.7",
    version: "0.3.7",
    publishedAt: "2026-09-14T22:12:07Z",
    htmlUrl: "https://github.com/Shadelight/ia-usage-bar/releases/tag/v0.3.7",
    windowsExe: "https://example.test/setup.exe",
  };
  storage.setItem(CACHE_KEY, JSON.stringify({ savedAt: 1_000, release: cached }));
  let fetches = 0;
  const release = await getLatestRelease({
    fetchFn: async () => {
      fetches += 1;
      throw new Error("should not fetch");
    },
    storage,
    now: 1_000 + 60_000,
  });
  assert.equal(fetches, 0);
  assert.equal(release.windowsExe, "https://example.test/setup.exe");
  assert.equal(release.ok, true);
});

test("getLatestRelease hydrates from the GitHub API payload", async () => {
  const storage = memoryStorage();
  const release = await getLatestRelease({
    fetchFn: async () => ({
      ok: true,
      json: async () => ({
        tag_name: "v0.3.7",
        published_at: "2026-09-14T22:12:07Z",
        html_url: "https://github.com/Shadelight/ia-usage-bar/releases/tag/v0.3.7",
        assets: FIXTURE_ASSETS,
      }),
    }),
    storage,
    skipSnapshot: true,
    now: 50,
  });
  assert.equal(release.ok, true);
  assert.equal(release.androidApk, "https://example.test/app.apk");
  const stored = JSON.parse(storage.getItem(CACHE_KEY));
  assert.equal(stored.release.version, "0.3.7");
});

test("getLatestRelease uses the baked snapshot when the API is down", async () => {
  const storage = memoryStorage();
  const release = await getLatestRelease({
    fetchFn: async (url) => {
      if (String(url).includes("api.github.com")) throw new Error("api down");
      return {
        ok: true,
        json: async () => ({
          ok: true,
          tag: "v0.3.7",
          version: "0.3.7",
          publishedAt: "2026-09-14T22:12:07Z",
          htmlUrl: "https://github.com/Shadelight/ia-usage-bar/releases/tag/v0.3.7",
          windowsExe: "https://example.test/setup.exe",
        }),
      };
    },
    storage,
    now: 1,
  });
  assert.equal(release.ok, true);
  assert.equal(release.windowsExe, "https://example.test/setup.exe");
});

function memoryStorage() {
  const data = new Map();
  return {
    getItem: (key) => (data.has(key) ? data.get(key) : null),
    setItem: (key, value) => {
      data.set(key, String(value));
    },
  };
}
