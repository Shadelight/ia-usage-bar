import { PROVIDERS, iconSrc, monogramDataUri } from "./providers.js";
import { downloadUrl, getLatestRelease } from "./releases.js";

export const OPENVSX_URL = "https://open-vsx.org/extension/shadelightdev/ia-usage";
export const SITE_URL = "https://shadelight.github.io/ia-usage-bar/";

export function resolveLang({ search = "", saved = null, browser = "en" } = {}) {
  const query = new URLSearchParams(search.startsWith("?") ? search.slice(1) : search).get("lang");
  if (query === "en" || query === "es") return query;
  if (saved === "en" || saved === "es") return saved;
  return String(browser).toLowerCase().startsWith("es") ? "es" : "en";
}

export const COPY = {
  en: {
    menuOpen: "Open menu",
    menuClose: "Close menu",
    navFeatures: "Features",
    navProviders: "Providers",
    navPrivacy: "Privacy",
    navGithub: "GitHub",
    navDownload: "Download",
    themeToggle: "Toggle theme",
    pageTitle: "IA Usage – Claude, ChatGPT, Codex & Cursor Usage Monitor",
    pageDescription:
      "Track Claude Code, ChatGPT Codex, Cursor, Antigravity, Copilot and other AI usage limits, quotas and reset times from Windows, Android and VS Code.",
    heroKicker: "AI quota monitor",
    heroTitle: "All your AI limits. One <span>experience</span>.",
    heroSubtitle:
      "Monitor quotas, resets, availability, and usage for the AI tools you actually use — from Windows, Android, and VS Code.",
    ctaPrimary: "Download for Windows",
    ctaSecondary: "View on GitHub",
    finePrint: "Free · Open source · Windows 10/11 · Android · VS Code",
    benefit1t: "Total control",
    benefit1p: "Your quotas and limits in one place.",
    benefit2t: "Useful alerts",
    benefit2p: "Stay ahead of limits, resets, and exhaustion.",
    benefit3t: "Private and local-first",
    benefit3p: "Your credentials stay on your devices.",
    platformsKicker: "Also available on",
    platformsTitle: "Same quotas, wherever you work",
    platformsSubtitle:
      "IA Usage Desktop is the main hub. Android and VS Code keep your limits visible while you work or you're away from the desk.",
    platformWindowsBadge: "Primary",
    platformWindowsP:
      "Tray app with quota panel, recommendations, alerts, compact mode, and a detailed view. Windows 10/11.",
    platformWindowsCta: "Download .exe",
    platformWindowsMsi: ".msi",
    platformAndroidP:
      "Companion app securely paired to your PC, with a dashboard, alerts, updates, and home-screen widgets so you can check quotas from the home screen.",
    platformAndroidCta: "Download APK",
    platformVscodeP:
      "Your limits directly in the editor status bar. Compatible with VS Code and compatible forks such as Cursor, Windsurf, and Antigravity.",
    platformVscodeCta: "Download .vsix",
    platformVscodeOpenVsx: "Open VSX",
    latestVersion: "Latest version",
    tourTitle: "Everything in sync",
    tourSubtitle: "One product, three surfaces — desktop, editor, and phone.",
    providersEyebrow: "Compatible providers",
    providersTitle: "One app, every provider you pay for",
    providersSubtitle: "Enable only what you use. IA Usage never disables a provider for you.",
    providersMore: "+ more via API key or local session",
    privacyKicker: "Privacy",
    privacyTitle: "Your credentials stay on your devices",
    privacySubtitle:
      "IA Usage reuses sessions and credentials that already exist locally when a provider allows it — instead of asking you to sign in again.",
    privacyLi1: "Manually entered API keys are stored in the OS credential store, never in plaintext config.",
    privacyLi2: "The app never stores passwords.",
    privacyLi3: "Processing is local, except the calls each enabled provider needs to its own usage endpoint.",
    privacyLi4: "No telemetry, no analytics.",
    privacyLink: "Full policy in SECURITY.md →",
    privacyRow1k: "Provider session",
    privacyRow1v: "Managed by the original CLI/app, read locally",
    privacyRow2k: "Manual API keys",
    privacyRow2v: "OS credential store",
    privacyRow3k: "Configuration",
    privacyRow3v: "Local only",
    privacyRow4k: "Telemetry",
    privacyRow4vNone: "None",
    finalTitle: "Always download the latest version",
    finalSubtitle: "Windows, Android, and VS Code are served from the latest published release.",
    finalWindows: "Windows",
    finalAndroid: "Android",
    finalVscode: "VS Code",
    footerNote: "Not affiliated with Anthropic, OpenAI, Cursor, or any listed provider.",
    footerCli: "CLI available in the repository for scripting and automation.",
    heroShotAlt: "IA Usage Desktop showing Claude Code, Codex, Cursor, and OpenAI API quotas",
    windowsShotAlt: "IA Usage Desktop quota dashboard on Windows",
    androidShotAlt: "IA Usage Android companion with home-screen quota widgets",
    vscodeShotAlt: "IA Usage VS Code extension status bar and quota tooltip",
    tourDesktopAlt: "IA Usage Desktop",
    tourVscodeAlt: "IA Usage in VS Code",
    tourAndroidAlt: "IA Usage Android widgets",
  },
  es: {
    menuOpen: "Abrir menú",
    menuClose: "Cerrar menú",
    navFeatures: "Funciones",
    navProviders: "Proveedores",
    navPrivacy: "Privacidad",
    navGithub: "GitHub",
    navDownload: "Descargar",
    themeToggle: "Cambiar tema",
    pageTitle: "IA Usage – Monitor de uso de Claude, ChatGPT, Codex y Cursor",
    pageDescription:
      "Controla cuotas, límites y reinicios de Claude Code, ChatGPT Codex, Cursor, Antigravity, Copilot y otras IAs desde Windows, Android y VS Code.",
    heroKicker: "Monitor de cuotas de IA",
    heroTitle: "Todos tus límites de IA. Una sola <span>experiencia</span>.",
    heroSubtitle:
      "Monitoriza cuotas, reinicios, disponibilidad y uso de las herramientas de IA que realmente utilizas desde Windows, Android y VS Code.",
    ctaPrimary: "Descargar para Windows",
    ctaSecondary: "Ver en GitHub",
    finePrint: "Gratis · Open source · Windows 10/11 · Android · VS Code",
    benefit1t: "Control total",
    benefit1p: "Tus cuotas y límites en un mismo lugar.",
    benefit2t: "Alertas útiles",
    benefit2p: "Anticípate a límites, resets y agotamientos.",
    benefit3t: "Privado y local-first",
    benefit3p: "Tus credenciales permanecen en tus dispositivos.",
    platformsKicker: "También disponible en",
    platformsTitle: "Las mismas cuotas, dondequiera que trabajes",
    platformsSubtitle:
      "IA Usage Desktop es el centro principal. Android y VS Code mantienen tus límites visibles mientras trabajas o estás lejos del escritorio.",
    platformWindowsBadge: "Principal",
    platformWindowsP:
      "App de bandeja con panel de cuotas, recomendaciones, alertas, modo compacto y vista detallada. Windows 10/11.",
    platformWindowsCta: "Descargar .exe",
    platformWindowsMsi: ".msi",
    platformAndroidP:
      "App companion vinculada de forma segura al PC, con dashboard, alertas, actualizaciones y widgets para consultar las cuotas desde la pantalla de inicio.",
    platformAndroidCta: "Descargar APK",
    platformVscodeP:
      "Tus límites directamente en la barra de estado del editor. Compatible con VS Code y forks compatibles como Cursor, Windsurf y Antigravity.",
    platformVscodeCta: "Descargar .vsix",
    platformVscodeOpenVsx: "Open VSX",
    latestVersion: "Última versión",
    tourTitle: "Todo sincronizado",
    tourSubtitle: "Un solo producto, tres superficies: escritorio, editor y teléfono.",
    providersEyebrow: "Proveedores compatibles",
    providersTitle: "Una app, todos los proveedores que pagas",
    providersSubtitle: "Activa solo los que usas. IA Usage nunca desactiva un proveedor por ti.",
    providersMore: "+ más vía clave API o sesión local",
    privacyKicker: "Privacidad",
    privacyTitle: "Tus credenciales se quedan en tus dispositivos",
    privacySubtitle:
      "IA Usage reutiliza sesiones y credenciales que ya existen localmente cuando el proveedor lo permite, en vez de pedirte que vuelvas a autenticarte.",
    privacyLi1: "Las claves API añadidas manualmente se guardan en el almacén de credenciales del sistema, nunca en texto plano.",
    privacyLi2: "La app nunca almacena contraseñas.",
    privacyLi3: "El procesamiento es local, salvo las llamadas que cada proveedor activo necesita a su propio endpoint de uso.",
    privacyLi4: "Sin telemetría, sin analítica.",
    privacyLink: "Política completa en SECURITY.md →",
    privacyRow1k: "Sesión del proveedor",
    privacyRow1v: "La gestiona la app/CLI original, leída localmente",
    privacyRow2k: "Claves API manuales",
    privacyRow2v: "Almacén de credenciales del sistema",
    privacyRow3k: "Configuración",
    privacyRow3v: "Solo local",
    privacyRow4k: "Telemetría",
    privacyRow4vNone: "Ninguna",
    finalTitle: "Descarga siempre la última versión",
    finalSubtitle: "Windows, Android y VS Code se sirven desde la última release publicada.",
    finalWindows: "Windows",
    finalAndroid: "Android",
    finalVscode: "VS Code",
    footerNote: "No afiliado con Anthropic, OpenAI, Cursor ni ningún proveedor listado.",
    footerCli: "CLI disponible en el repositorio para scripts y automatización.",
    heroShotAlt: "IA Usage Desktop mostrando cuotas de Claude Code, Codex, Cursor y OpenAI API",
    windowsShotAlt: "Panel de cuotas de IA Usage Desktop en Windows",
    androidShotAlt: "Companion Android de IA Usage con widgets de cuota en la pantalla de inicio",
    vscodeShotAlt: "Extensión IA Usage en VS Code con barra de estado y tooltip de cuotas",
    tourDesktopAlt: "IA Usage Desktop",
    tourVscodeAlt: "IA Usage en VS Code",
    tourAndroidAlt: "Widgets de IA Usage en Android",
  },
};

let currentLang = "es";

function escapeHtml(value) {
  return String(value).replace(/[&<>"']/g, (char) => ({
    "&": "&amp;",
    "<": "&lt;",
    ">": "&gt;",
    '"': "&quot;",
    "'": "&#39;",
  }[char]));
}

function updateMenuLabel() {
  const btn = document.querySelector("[data-menu-btn]");
  if (!btn) return;
  const dict = COPY[currentLang] || COPY.en;
  const open = btn.getAttribute("aria-expanded") === "true";
  btn.setAttribute("aria-label", open ? dict.menuClose : dict.menuOpen);
}

export function applyCopy(lang) {
  currentLang = lang;
  const dict = COPY[lang] || COPY.en;
  document.querySelectorAll("[data-i18n]").forEach((el) => {
    const key = el.getAttribute("data-i18n");
    if (dict[key] !== undefined) el.innerHTML = dict[key];
  });
  document.querySelectorAll("[data-i18n-alt]").forEach((el) => {
    const key = el.getAttribute("data-i18n-alt");
    if (dict[key] !== undefined) el.setAttribute("alt", dict[key]);
  });
  document.querySelectorAll("[data-i18n-aria]").forEach((el) => {
    const key = el.getAttribute("data-i18n-aria");
    if (dict[key] !== undefined) el.setAttribute("aria-label", dict[key]);
  });
  updateMenuLabel();
  document.documentElement.lang = lang;
  if (dict.pageTitle) document.title = dict.pageTitle;
  setMeta('meta[name="description"]', "content", dict.pageDescription);
  setMeta('meta[property="og:title"]', "content", dict.pageTitle);
  setMeta('meta[property="og:description"]', "content", dict.pageDescription);
  setMeta('meta[property="og:locale"]', "content", lang === "es" ? "es_ES" : "en_US");
  setMeta('meta[name="twitter:title"]', "content", dict.pageTitle);
  setMeta('meta[name="twitter:description"]', "content", dict.pageDescription);
  document.querySelectorAll("[data-lang-btn]").forEach((btn) => {
    btn.classList.toggle("active", btn.getAttribute("data-lang-btn") === lang);
  });
  try {
    localStorage.setItem("iausage-lang", lang);
  } catch (_) {
    /* private mode */
  }
}

function setMeta(selector, attr, value) {
  if (!value) return;
  const el = document.querySelector(selector);
  if (el) el.setAttribute(attr, value);
}

function syncLangUrl(lang) {
  if (typeof history === "undefined" || !history.replaceState) return;
  try {
    const url = new URL(window.location.href);
    url.searchParams.set("lang", lang);
    history.replaceState({}, "", url);
  } catch (_) {
    /* file:// previews */
  }
}

function initLang() {
  let saved = null;
  try {
    saved = localStorage.getItem("iausage-lang");
  } catch (_) {
    saved = null;
  }
  const lang = resolveLang({
    search: typeof location !== "undefined" ? location.search : "",
    saved,
    browser: typeof navigator !== "undefined" ? navigator.language : "en",
  });
  applyCopy(lang);
  document.querySelectorAll("[data-lang-btn]").forEach((btn) => {
    btn.addEventListener("click", () => {
      const next = btn.getAttribute("data-lang-btn");
      applyCopy(next);
      syncLangUrl(next);
    });
  });
}

function initTheme() {
  const root = document.documentElement;
  const btn = document.querySelector("[data-theme-btn]");
  let saved = null;
  try {
    saved = localStorage.getItem("iausage-theme");
  } catch (_) {
    /* private mode */
  }
  if (saved) root.setAttribute("data-theme", saved);
  if (btn) {
    btn.addEventListener("click", () => {
      const current = root.getAttribute("data-theme") || (matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light");
      const next = current === "dark" ? "light" : "dark";
      root.setAttribute("data-theme", next);
      try {
        localStorage.setItem("iausage-theme", next);
      } catch (_) {
        /* private mode */
      }
    });
  }
}

function initMenu() {
  const btn = document.querySelector("[data-menu-btn]");
  const header = document.getElementById("site-nav");
  const panel = document.getElementById("mobile-menu");
  if (!btn || !header || !panel) return;
  const setOpen = (open) => {
    btn.setAttribute("aria-expanded", String(open));
    header.classList.toggle("nav-open", open);
    updateMenuLabel();
  };
  btn.addEventListener("click", () => {
    setOpen(btn.getAttribute("aria-expanded") !== "true");
  });
  panel.addEventListener("click", (e) => {
    if (e.target.closest("a")) setOpen(false);
  });
  document.addEventListener("keydown", (e) => {
    if (e.key === "Escape") setOpen(false);
  });
  document.addEventListener("click", (e) => {
    if (header.classList.contains("nav-open") && !header.contains(e.target)) setOpen(false);
  });
  window.addEventListener("resize", () => {
    if (window.innerWidth > 860) setOpen(false);
  });
}

function renderProviders() {
  const grid = document.querySelector("[data-provider-grid]");
  if (!grid) return;
  grid.innerHTML = PROVIDERS.map((provider) => {
    const src = iconSrc(provider) || monogramDataUri(provider.name);
    return `<span class="chip" title="${escapeHtml(provider.name)}"><img src="${escapeHtml(src)}" alt="" width="18" height="18">${escapeHtml(provider.name)}</span>`;
  }).join("");
}

export function applyRelease(release) {
  document.querySelectorAll("[data-asset]").forEach((el) => {
    const key = el.getAttribute("data-asset");
    el.setAttribute("href", downloadUrl(release, key));
  });
  const version = release?.ok && release.version ? `v${String(release.version).replace(/^v/i, "")}` : "";
  document.querySelectorAll("[data-release-meta]").forEach((el) => {
    el.hidden = !version;
  });
  document.querySelectorAll("[data-release-version]").forEach((el) => {
    el.textContent = version;
  });
}

async function initDownloads() {
  const release = await getLatestRelease();
  applyRelease(release);
}

function hideBrokenProductImages() {
  document.querySelectorAll("img[data-optional-shot]").forEach((img) => {
    const hide = () => {
      img.hidden = true;
      img.closest(".shot-frame, .tour-piece")?.classList.add("shot-missing");
    };
    img.addEventListener("error", hide);
    if (img.complete && img.naturalWidth === 0) hide();
  });
}

if (typeof document !== "undefined") {
  document.addEventListener("DOMContentLoaded", () => {
    initLang();
    initTheme();
    initMenu();
    renderProviders();
    applyCopy(currentLang);
    hideBrokenProductImages();
    initDownloads();
    const yearEl = document.querySelector("[data-year]");
    if (yearEl) yearEl.textContent = String(new Date().getFullYear());
  });
}
