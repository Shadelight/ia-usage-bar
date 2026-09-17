/** Single source of truth for landing download URLs. No hardcoded versions. */

export const REPO = "Shadelight/ia-usage-bar";
export const FALLBACK_RELEASE_URL = `https://github.com/${REPO}/releases/latest`;
export const GITHUB_API_LATEST = `https://api.github.com/repos/${REPO}/releases/latest`;
export const SNAPSHOT_URL = "./latest-release.json";
export const CACHE_KEY = "iausage-latest-release";
export const CACHE_TTL_MS = 10 * 60 * 1000;

const IGNORE_NAME =
  /(\.aab$|\.zip$|\.tar\.gz$|\.tgz$|^source\.|\.sha256$|\.sig$|\.blockmap$)/i;

/**
 * @typedef {object} ReleaseAsset
 * @property {string} name
 * @property {string} [browser_download_url]
 * @property {string} [url]
 */

/**
 * @typedef {object} LatestRelease
 * @property {boolean} ok
 * @property {string} tag
 * @property {string} version
 * @property {string} publishedAt
 * @property {string} htmlUrl
 * @property {string} [windowsExe]
 * @property {string} [windowsMsi]
 * @property {string} [androidApk]
 * @property {string} [vscodeVsix]
 * @property {string} [checksums]
 */

/** @returns {LatestRelease} */
export function fallbackRelease() {
  return {
    ok: false,
    tag: "",
    version: "",
    publishedAt: "",
    htmlUrl: FALLBACK_RELEASE_URL,
  };
}

function assetUrl(asset) {
  return (asset && (asset.browser_download_url || asset.url)) || "";
}

function usable(assets, test) {
  return (assets || []).filter((asset) => {
    const name = String(asset?.name || "");
    if (!name || IGNORE_NAME.test(name)) return false;
    return test(name);
  });
}

/**
 * Pick download URLs from a GitHub release asset list by extension/pattern.
 * Never returns source archives, AABs, or checksum companions as installers.
 * @param {ReleaseAsset[]} assets
 */
export function resolveReleaseAssets(assets) {
  const list = Array.isArray(assets) ? assets : [];
  const exes = usable(list, (name) => /\.exe$/i.test(name));
  const setup = exes.find((asset) => /setup/i.test(asset.name));
  const msis = usable(list, (name) => /\.msi$/i.test(name));
  const apks = usable(list, (name) => /\.apk$/i.test(name));
  const vsixes = usable(list, (name) => /\.vsix$/i.test(name));
  const sums = (list || []).find((asset) => /^SHA256SUMS\.txt$/i.test(String(asset?.name || "")));

  /** @type {Record<string, string>} */
  const out = {};
  const exe = setup || exes[0];
  if (exe && assetUrl(exe)) out.windowsExe = assetUrl(exe);
  if (msis[0] && assetUrl(msis[0])) out.windowsMsi = assetUrl(msis[0]);
  if (apks[0] && assetUrl(apks[0])) out.androidApk = assetUrl(apks[0]);
  if (vsixes[0] && assetUrl(vsixes[0])) out.vscodeVsix = assetUrl(vsixes[0]);
  if (sums && assetUrl(sums)) out.checksums = assetUrl(sums);
  return out;
}

/**
 * @param {object} payload GitHub API release JSON or an already-normalized LatestRelease
 * @returns {LatestRelease}
 */
export function parseGithubRelease(payload) {
  if (!payload || typeof payload !== "object") return fallbackRelease();
  if (payload.ok === true && payload.htmlUrl) {
    return {
      ok: true,
      tag: String(payload.tag || ""),
      version: String(payload.version || String(payload.tag || "").replace(/^v/i, "")),
      publishedAt: String(payload.publishedAt || ""),
      htmlUrl: String(payload.htmlUrl),
      windowsExe: payload.windowsExe || undefined,
      windowsMsi: payload.windowsMsi || undefined,
      androidApk: payload.androidApk || undefined,
      vscodeVsix: payload.vscodeVsix || undefined,
      checksums: payload.checksums || undefined,
    };
  }

  const tag = String(payload.tag_name || payload.tag || "").trim();
  const resolved = resolveReleaseAssets(payload.assets);
  const version = tag.replace(/^v/i, "");
  const htmlUrl = String(payload.html_url || payload.htmlUrl || FALLBACK_RELEASE_URL);
  const hasAsset = Boolean(
    resolved.windowsExe || resolved.windowsMsi || resolved.androidApk || resolved.vscodeVsix,
  );
  if (!tag && !hasAsset) return fallbackRelease();

  return {
    ok: true,
    tag,
    version,
    publishedAt: String(payload.published_at || payload.publishedAt || ""),
    htmlUrl: htmlUrl || FALLBACK_RELEASE_URL,
    ...resolved,
  };
}

/**
 * URL for a named asset. Missing assets fall back to the release page, never "#" or empty.
 * @param {LatestRelease | null | undefined} release
 * @param {keyof LatestRelease} key
 */
export function downloadUrl(release, key) {
  const value = release && typeof release === "object" ? release[key] : "";
  if (typeof value === "string" && /^https?:\/\//i.test(value)) return value;
  const htmlUrl = release && release.htmlUrl;
  if (typeof htmlUrl === "string" && /^https?:\/\//i.test(htmlUrl)) return htmlUrl;
  return FALLBACK_RELEASE_URL;
}

function readCache(storage, now) {
  if (!storage) return null;
  try {
    const raw = storage.getItem(CACHE_KEY);
    if (!raw) return null;
    const parsed = JSON.parse(raw);
    if (!parsed || typeof parsed.savedAt !== "number" || !parsed.release) return null;
    if (now - parsed.savedAt > CACHE_TTL_MS) return null;
    const release = parseGithubRelease(parsed.release);
    return release.ok ? release : null;
  } catch {
    return null;
  }
}

function writeCache(storage, release, now) {
  if (!storage || !release?.ok) return;
  try {
    storage.setItem(CACHE_KEY, JSON.stringify({ savedAt: now, release }));
  } catch {
    /* quota / private mode */
  }
}

async function fetchJson(fetchFn, url, headers) {
  const response = await fetchFn(url, { headers });
  if (!response || !response.ok) throw new Error(`HTTP ${response && response.status}`);
  return response.json();
}

/**
 * Resolve the latest published release. Never throws; always returns a safe object.
 * @param {object} [options]
 * @param {typeof fetch} [options.fetchFn]
 * @param {{ getItem: Function, setItem: Function } | null} [options.storage]
 * @param {number} [options.now]
 * @param {string} [options.snapshotUrl]
 * @param {string} [options.apiUrl]
 * @param {boolean} [options.skipSnapshot]
 * @param {boolean} [options.skipApi]
 * @returns {Promise<LatestRelease>}
 */
export async function getLatestRelease(options = {}) {
  const fetchFn = options.fetchFn || globalThis.fetch;
  const storage = options.storage === undefined
    ? (typeof localStorage !== "undefined" ? localStorage : null)
    : options.storage;
  const now = options.now ?? Date.now();
  const snapshotUrl = options.snapshotUrl ?? SNAPSHOT_URL;
  const apiUrl = options.apiUrl ?? GITHUB_API_LATEST;

  const cached = readCache(storage, now);
  if (cached) return cached;

  if (!options.skipApi && typeof fetchFn === "function") {
    try {
      const payload = await fetchJson(fetchFn, apiUrl, {
        Accept: "application/vnd.github+json",
        "User-Agent": "IA-Usage-Landing",
      });
      const release = parseGithubRelease(payload);
      if (release.ok) {
        writeCache(storage, release, now);
        return release;
      }
    } catch {
      /* try snapshot, then fallback */
    }
  }

  if (!options.skipSnapshot && typeof fetchFn === "function") {
    try {
      const payload = await fetchJson(fetchFn, snapshotUrl, { Accept: "application/json" });
      const release = parseGithubRelease(payload);
      if (release.ok) {
        writeCache(storage, release, now);
        return release;
      }
    } catch {
      /* fallback */
    }
  }

  return fallbackRelease();
}
