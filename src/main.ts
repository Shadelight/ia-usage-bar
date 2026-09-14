import "./styles.css";

import { $, escapeHtml, invokeCmd, isTauri } from "./api";
import type { Dashboard, ProviderStatus, SyncPairingDto, SyncStatusDto } from "./api";
import { sanitizeTechnicalDetails, shouldShowRecoveryToast } from "./errors";
import { setLang, t } from "./i18n";
import { buildSanitizedDiagnosis, renderDash, updateLoadingClocks, updateResetClocks } from "./views/dash";
import { actionIconSvg } from "./provider-actions";
import { checkForUpdates, clearCredentialDraft, focusProviderInSettings, isCredentialValidating, patchCredentialEye, patchProviderInteractiveState, patchSettings, persistConfig, refreshSyncView, reloadCliStatus, renderSettings, setCredentialDraft, setCredentialValidating, setExpandedProvider, setSavingProvider, setSyncPairing, setSyncPassphraseDraft, startPairingCountdown, toggleCredentialRevealed } from "./views/settings";
import type { SettingsCategory } from "./views/settings";
import { previewDashboard, renderSpend } from "./views/spend";

let dash: Dashboard | null = null;
let selectedId = localStorage.getItem("selected") || "";
let settingsCategory = (localStorage.getItem("settingsCategory") as SettingsCategory) || "general";
let view: "dash" | "spend" | "settings" = "dash";
const previousStatuses = new Map<string, ProviderStatus>();
let toastTimer: number | undefined;
const uiStarted = performance.now();
let refreshWhenFocused: string | null = null;
// Dashboard events are asynchronous and can describe the state from before a
// command finished. Keep local intent authoritative until Rust accepts or
// rejects it, without ever replacing the control the user is touching.
const pendingProviderEnabled = new Map<string, boolean>();
const pendingSourcePreferences = new Map<string, string | null>();
// Optimistic credential presence bridges the command response and its catalog
// event. It is bounded: if a confirming dashboard never arrives, the actual
// backend value wins instead of showing a saved-credential badge forever.
const pendingCredentialPresence = new Map<string, boolean>();
const pendingCredentialPresenceExpiry = new Map<string, number>();
const CREDENTIAL_PRESENCE_GRACE_MS = 15_000;

function scheduleCredentialPresenceExpiry(id: string): void {
  const expiresAt = performance.now() + CREDENTIAL_PRESENCE_GRACE_MS;
  pendingCredentialPresenceExpiry.set(id, expiresAt);
  window.setTimeout(() => {
    // A newer save/delete has its own deadline. Otherwise discard the local
    // claim and ask Rust for the authoritative catalog so the badge cannot
    // survive forever if an event was lost.
    if (pendingCredentialPresenceExpiry.get(id) !== expiresAt) return;
    pendingCredentialPresence.delete(id);
    pendingCredentialPresenceExpiry.delete(id);
    void invokeCmd<Dashboard>("get_dashboard").then((result) => {
      if (result.ok) void applyDashboard(result.value);
    });
  }, CREDENTIAL_PRESENCE_GRACE_MS);
}
// Tracks scoped validation refreshes that already emitted their loading
// state, so a stale pre-save dashboard can never end "Validando…" early.
const credentialValidationObserved = new Set<string>();

type AppTheme = "system" | "light" | "dark";

function applyTheme(theme = (localStorage.getItem("theme") as AppTheme | null) || "system"): void {
  const effective = theme === "system"
    ? (window.matchMedia("(prefers-color-scheme: light)").matches ? "light" : "dark")
    : theme;
  document.documentElement.dataset.theme = effective;
  localStorage.setItem("theme", theme);
}

async function openExternal(url: string): Promise<void> {
  if (!url.startsWith("https://")) return;
  try {
    if (!isTauri()) {
      window.open(url, "_blank", "noopener,noreferrer");
      return;
    }
    const { openUrl } = await import("@tauri-apps/plugin-opener");
    await openUrl(url);
  } catch (error) {
    const detail = error instanceof Error ? error.message : String(error);
    window.dispatchEvent(new CustomEvent("app-command-error", { detail }));
  }
}

function showToast(message: string, durationMs = 2500): void {
  const toast = $("status-toast");
  toast.textContent = message;
  toast.classList.remove("hidden", "status-toast-error");
  window.clearTimeout(toastTimer);
  toastTimer = window.setTimeout(() => toast.classList.add("hidden"), durationMs);
}

function showRecoveryToast(name: string): void {
  showToast(t("recovered").replace("{name}", name), 3500);
}

async function copyToClipboard(text: string): Promise<boolean> {
  try {
    if (navigator.clipboard?.writeText) {
      await navigator.clipboard.writeText(text);
      return true;
    }
    const area = document.createElement("textarea");
    area.value = text;
    area.style.position = "fixed";
    area.style.opacity = "0";
    document.body.appendChild(area);
    area.select();
    const success = document.execCommand("copy");
    document.body.removeChild(area);
    return success;
  } catch {
    return false;
  }
}

function showCommandError(detail: string): void {
  const safeDetail = sanitizeTechnicalDetails(detail);
  console.error(safeDetail);
  const toast = $("status-toast");
  toast.textContent = safeDetail || t("commandFailed");
  toast.classList.add("status-toast-error");
  toast.classList.remove("hidden");
  window.clearTimeout(toastTimer);
  toastTimer = window.setTimeout(() => {
    toast.classList.add("hidden");
    toast.classList.remove("status-toast-error");
  }, 5_000);
}

function renderBootstrapError(): void {
  $("view-dash").innerHTML = `<div class="empty"><p>${t("bootstrapFailed")}</p><button data-act="refresh">${t("retry")}</button></div>`;
}

function watchRecoveries(next: Dashboard): void {
  for (const provider of next.providers) {
    const previous = previousStatuses.get(provider.id);
    if (shouldShowRecoveryToast(previous, provider.status)) showRecoveryToast(provider.name);
    previousStatuses.set(provider.id, provider.status);
  }
}

function closeHeadMenu(): void {
  $("head-menu").classList.add("hidden");
}

// El menú superior navega a secciones de Ajustes/vistas: nunca muta estado
// directamente (la única excepción es el toggle de Modo compacto).
function openSettingsCategory(category: SettingsCategory): void {
  view = "settings";
  settingsCategory = category;
  localStorage.setItem("settingsCategory", settingsCategory);
  renderSettings(dash, settingsCategory);
  paintDash();
}

function paintHeadMenu() {
  $("btn-pin").classList.toggle("active", !!dash?.alwaysOnTop);
  const rows = [
    `<button data-act="compact"><span>${t("compactMode")}</span>${dash?.compactMode ? `<span class="menu-check">${actionIconSvg("check", 13)}</span>` : ""}</button>`,
    `<button data-act="settings-notifications"><span class="menu-label">${actionIconSvg("bell", 14)}<span>${t("notifications")}</span></span></button>`,
    `<button data-act="spend"><span>${t("spend")}</span></button>`,
    `<button data-act="pair-phone"><span class="menu-label">${actionIconSvg("phone", 14)}<span>${t("syncPhoneMenu")}</span></span></button>`,
    `<button data-act="settings-data"><span class="menu-label">${actionIconSvg("folder", 14)}<span>${t("settingsData")}</span></span></button>`,
    `<button data-act="settings-general"><span class="menu-label">${actionIconSvg("settings", 14)}<span>${t("settings")}</span></span></button>`,
  ].filter(Boolean);
  $("head-menu").innerHTML = rows.join("");
}

async function paintAppVersion(): Promise<void> {
  const footer = document.getElementById("app-version");
  if (!footer) return;
  footer.textContent = "IA Usage";
  if (!isTauri()) return;
  try {
    const { getVersion } = await import("@tauri-apps/api/app");
    footer.textContent = `IA Usage · v${await getVersion()}`;
  } catch {
    // Browser/dev fallback stays as the product name only.
  }
}

function paintDash() {
  selectedId = renderDash(dash, selectedId);
  $("view-dash").classList.toggle("hidden", view !== "dash");
  $("view-spend").classList.toggle("hidden", view !== "spend");
  $("view-settings").classList.toggle("hidden", view !== "settings");
  paintHeadMenu();
  document.getElementById("panel")?.classList.toggle("compact", !!dash?.compactMode);
}

async function applyDashboard(d: Dashboard) {
  const compactChanged = dash?.compactMode !== d.compactMode;
  for (const vendor of d.catalog) {
    // Compare-first: inspect the value Rust just sent, then apply the
    // optimistic one only while the dashboard is still older than the
    // command. Assigning first and comparing after would always match.
    const pendingEnabled = pendingProviderEnabled.get(vendor.id);
    if (pendingEnabled !== undefined) {
      if (vendor.enabled === pendingEnabled) pendingProviderEnabled.delete(vendor.id);
      else vendor.enabled = pendingEnabled;
    }
    if (pendingSourcePreferences.has(vendor.id)) {
      const pendingSource = pendingSourcePreferences.get(vendor.id) ?? null;
      if (vendor.sourcePreference === pendingSource) pendingSourcePreferences.delete(vendor.id);
      else vendor.sourcePreference = pendingSource;
    }
    const pendingCredential = pendingCredentialPresence.get(vendor.id);
    if (pendingCredential !== undefined) {
      if (vendor.hasCredential === pendingCredential) {
        pendingCredentialPresence.delete(vendor.id);
        pendingCredentialPresenceExpiry.delete(vendor.id);
      } else if ((pendingCredentialPresenceExpiry.get(vendor.id) ?? 0) > performance.now()) {
        vendor.hasCredential = pendingCredential;
      } else {
        pendingCredentialPresence.delete(vendor.id);
        pendingCredentialPresenceExpiry.delete(vendor.id);
      }
    }
    // End "Validando…" only once the scoped validation refresh observed its
    // loading state and left it. A stale pre-save dashboard (no loading)
    // must never clear it early.
    if (isCredentialValidating(vendor.id)) {
      if (d.loadingProviders.includes(vendor.id)) {
        credentialValidationObserved.add(vendor.id);
      } else if (credentialValidationObserved.has(vendor.id)) {
        credentialValidationObserved.delete(vendor.id);
        setCredentialValidating(vendor.id, false);
      }
    }
  }
  watchRecoveries(d);
  dash = d;
  if (view === "settings") {
    // Never replace Settings as a side effect of a background refresh. In
    // particular, Windows closes an open <select> when its node is removed.
    patchSettings(dash, settingsCategory);
  }
  if (view === "spend") renderSpend(dash);
  paintDash();
  if (compactChanged) await applyWindowMode(d.compactMode);
  if (import.meta.env.DEV) console.debug(`dashboard rendered: ${Math.round(performance.now() - uiStarted)} ms`, { loading: d.loadingProviders });
}

async function applyWindowMode(compact: boolean) {
  if (!isTauri()) return;
  try {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    const { LogicalSize } = await import("@tauri-apps/api/dpi");
    const win = getCurrentWindow();
    if (compact) {
      if (window.innerHeight > 260) localStorage.setItem("normalWindowSize", JSON.stringify({ width: window.innerWidth, height: window.innerHeight }));
      await win.setMinSize(new LogicalSize(320, 180));
      await win.setSize(new LogicalSize(350, 220));
    } else {
      const saved = JSON.parse(localStorage.getItem("normalWindowSize") || "null") as { width?: number; height?: number } | null;
      await win.setMinSize(new LogicalSize(340, 420));
      if (window.innerHeight < 420) {
        await win.setSize(new LogicalSize(Math.max(340, saved?.width || 410), Math.max(420, saved?.height || 640)));
      }
    }
  } catch {
    /* ignore */
  }
}

/** The one "Vincular teléfono" flow, shared by the in-app menu and the tray
 * item — never duplicated between the two entry points. */
async function pairPhoneFlow(): Promise<void> {
  openSettingsCategory("sync");
  await refreshSyncView();
  const status = await invokeCmd<SyncStatusDto>("sync_get_status");
  if (!status.ok) {
    showCommandError(status.error ?? t("commandFailed"));
    return;
  }
  if (!status.value.lan) {
    showToast(t("syncPairNeedsLan"), 4000);
    requestAnimationFrame(() => document.getElementById("cfg-sync-lan")?.focus());
    return;
  }
  const pairing = await invokeCmd<SyncPairingDto>("sync_start_pairing");
  if (!pairing.ok) {
    showCommandError(pairing.error ?? t("commandFailed"));
    return;
  }
  setSyncPairing(pairing.value);
  await refreshSyncView();
  startPairingCountdown();
  requestAnimationFrame(() => {
    document.getElementById("sync-qr")?.scrollIntoView({ behavior: "smooth", block: "center" });
  });
}

async function handleAction(act: string) {
  if (act !== "toggle-menu") closeHeadMenu();
  switch (act) {
    case "toggle-menu":
      $("head-menu").classList.toggle("hidden");
      break;
    case "toggle-pin": {
      const enabled = !dash?.alwaysOnTop;
      await invokeCmd("set_always_on_top", { enabled });
      if (dash) dash.alwaysOnTop = enabled;
      paintHeadMenu();
      break;
    }
    case "compact": {
      const enabled = !dash?.compactMode;
      if (dash) dash.compactMode = enabled;
      view = "dash";
      paintDash();
      await applyWindowMode(enabled);
      await invokeCmd("set_compact_mode", { enabled });
      break;
    }
    case "toggle-notif": {
      // Compatibilidad: el menú ya no alterna notificaciones (navega a la
      // sección); el toggle vive en Ajustes → Notificaciones (cfg-notif).
      openSettingsCategory("notifications");
      break;
    }
    case "settings-notifications":
      openSettingsCategory("notifications");
      break;
    case "settings-data":
      openSettingsCategory("data");
      break;
    case "settings-sync":
      openSettingsCategory("sync");
      break;
    case "settings-general":
      openSettingsCategory("general");
      break;
    case "pair-phone":
      await pairPhoneFlow();
      break;
    case "open-logs":
      await invokeCmd("open_logs_folder");
      break;
    case "minimize":
      if (isTauri()) {
        const { getCurrentWindow } = await import("@tauri-apps/api/window");
        await getCurrentWindow().minimize();
      }
      break;
    case "close-window":
      if (isTauri()) {
        const { getCurrentWindow } = await import("@tauri-apps/api/window");
        await getCurrentWindow().close();
      }
      break;
    case "refresh":
      // The command starts background work, so make that work visible before
      // its first dashboard event reaches the webview.
      document.querySelectorAll<HTMLButtonElement>('[data-act="refresh"]').forEach((button) => {
        button.disabled = true;
        button.classList.add("is-refreshing");
        button.setAttribute("aria-busy", "true");
      });
      $("updated").textContent = t("updating");
      if (!(await invokeCmd("refresh_now")).ok && isTauri()) {
        document.querySelectorAll<HTMLButtonElement>('[data-act="refresh"]').forEach((button) => {
          button.disabled = false;
          button.classList.remove("is-refreshing");
          button.removeAttribute("aria-busy");
        });
      }
      break;
    case "detect":
      await invokeCmd("detect_providers");
      break;
    case "settings":
      openSettingsCategory("general");
      break;
    case "manage-providers":
      // The provider catalog is intentionally curated by VendorId::all();
      // “Add” means enable one of those supported integrations, rather than
      // pretending that an arbitrary custom provider can be queried.
      openSettingsCategory("providers");
      break;
    case "spend":
      view = "spend";
      renderSpend(dash);
      paintDash();
      break;
    case "back":
      view = "dash";
      paintDash();
      break;
    case "quit":
      await invokeCmd("quit");
      break;
    case "lang-en":
      setLang("en");
      paintDash();
      if (view === "settings") renderSettings(dash, settingsCategory);
      if (view === "spend") renderSpend(dash);
      break;
    case "lang-es":
      setLang("es");
      paintDash();
      if (view === "settings") renderSettings(dash, settingsCategory);
      if (view === "spend") renderSpend(dash);
      break;
    case "clear-logs":
      await invokeCmd("clear_logs");
      break;
    case "export-diagnostics":
      await invokeCmd("export_diagnostics");
      break;
    case "check-updates":
      await checkForUpdates();
      break;
  }
}

async function main() {
  applyTheme();
  void paintAppVersion();
  window.matchMedia("(prefers-color-scheme: light)").addEventListener("change", () => {
    if ((localStorage.getItem("theme") || "system") === "system") applyTheme("system");
  });
  if (import.meta.env.DEV) console.debug(`app UI mounted: ${Math.round(performance.now() - uiStarted)} ms`);
  window.addEventListener("app-command-error", (event) => {
    showCommandError((event as CustomEvent<string>).detail || "unknown command error");
  });
  window.addEventListener("focus", () => {
    const id = refreshWhenFocused;
    if (!id) return;
    refreshWhenFocused = null;
    void invokeCmd("refresh_provider", { id });
  });
  if (isTauri()) {
    try {
      const { getCurrentWindow } = await import("@tauri-apps/api/window");
      const win = getCurrentWindow();
      await win.show();
      await win.setFocus();
    } catch {
      /* ignore */
    }
  }

  document.addEventListener("click", async (e) => {
    const target = e.target as HTMLElement;
    const btn = target.closest<HTMLElement>("[data-act],[data-select],[data-savekey],[data-delkey],[data-toggle-key],[data-expand],[data-detect],[data-refresh-provider],[data-copy-cli],[data-setcat],[data-theme],[data-open-url],[data-install-update],[data-redeem-reset],[data-startpairing],[data-revokedevice],[data-syncexport],[data-repair-cli],[data-copy-cli-path],[data-test-cli]");
    if (!btn) {
      if (!target.closest("#head-menu, #btn-menu")) closeHeadMenu();
      return;
    }
    if (btn.dataset.act === "configure-provider") {
      const providerId = btn.dataset.providerId || selectedId;
      setExpandedProvider(providerId || null);
      openSettingsCategory("providers");
      focusProviderInSettings(providerId);
      return;
    }
    if (btn.dataset.act === "provider-login" && btn.dataset.providerId) {
      const result = await invokeCmd("start_provider_login", { id: btn.dataset.providerId });
      if (result.ok) showToast(t("loginStarted"));
      else showCommandError(result.error ?? t("commandFailed"));
      return;
    }
    if (btn.dataset.act === "copy-provider-diagnosis") {
      const providerId = btn.dataset.providerId || selectedId;
      const snapshot = dash?.providers.find((p) => p.id === providerId);
      const vendor = dash?.catalog.find((v) => v.id === providerId);
      if (snapshot && vendor) {
        const diag = buildSanitizedDiagnosis(snapshot, vendor);
        const copied = await copyToClipboard(diag);
        if (copied) showToast(t("copiedDiagnosis"));
      }
      return;
    }
    if (btn.dataset.act === "copy-cli-command") {
      const cmd = btn.dataset.cliCmd || `iausage usage ${selectedId}`;
      const copied = await copyToClipboard(cmd);
      if (copied) showToast(t("copiedCliCmd"));
      return;
    }
    if (btn.dataset.act) await handleAction(btn.dataset.act);
    if (btn.dataset.select) {
      selectedId = btn.dataset.select;
      localStorage.setItem("selected", selectedId);
      paintDash();
    }
    if (btn.dataset.setcat) {
      openSettingsCategory(btn.dataset.setcat as SettingsCategory);
    }
    if (btn.dataset.toggleKey) {
      // Eye toggle: pure DOM flip, never a re-render (the input keeps focus).
      const id = btn.dataset.toggleKey;
      const revealed = toggleCredentialRevealed(id);
      const input = document.querySelector<HTMLInputElement>(`[data-key="${id}"]`);
      if (input) input.type = revealed ? "text" : "password";
      btn.innerHTML = actionIconSvg(revealed ? "eye-off" : "eye", 16);
      btn.setAttribute("aria-label", t(revealed ? "hideCredential" : "showCredential"));
      btn.setAttribute("aria-pressed", revealed ? "true" : "false");
      return;
    }
    if (btn.dataset.savekey) {
      const id = btn.dataset.savekey;
      const input = document.querySelector<HTMLInputElement>(`[data-key="${id}"]`);
      const key = input?.value.trim() || "";
      if (!key) return;
      setCredentialDraft(id, input?.value || "");
      setSavingProvider(id);
      if (view === "settings") renderSettings(dash, settingsCategory);
      // Persistence and validation are two separate steps: save_api_key only
      // stores + emits the catalog, and the scoped refresh below starts
      // afterwards — so validation can never finish before the UI enters
      // "Validando…".
      const result = await invokeCmd("save_api_key", { id, key });
      setSavingProvider(null);
      if (result.ok || !isTauri()) {
        // Optimistic presence: the secret stays server-side only, so the
        // draft is dropped and the input renders empty with a saved badge.
        pendingCredentialPresence.set(id, true);
        scheduleCredentialPresenceExpiry(id);
        const vendor = dash?.catalog.find((v) => v.id === id);
        if (vendor) vendor.hasCredential = true;
        clearCredentialDraft(id);
        setCredentialValidating(id, true);
        if (view === "settings") renderSettings(dash, settingsCategory);
        await invokeCmd("refresh_provider", { id });
      } else {
        // Keep the draft: what the user typed is never discarded, and the
        // eye stays available on the non-empty input.
        showCommandError(result.error ?? t("commandFailed"));
        if (view === "settings") renderSettings(dash, settingsCategory);
      }
    }
    if (btn.dataset.delkey) {
      const id = btn.dataset.delkey;
      const result = await invokeCmd("delete_api_key", { id });
      if (result.ok || !isTauri()) {
        pendingCredentialPresence.set(id, false);
        scheduleCredentialPresenceExpiry(id);
        const vendor = dash?.catalog.find((v) => v.id === id);
        if (vendor) vendor.hasCredential = false;
        clearCredentialDraft(id);
        setCredentialValidating(id, false);
        credentialValidationObserved.delete(id);
        if (view === "settings") renderSettings(dash, settingsCategory);
        await invokeCmd("refresh_provider", { id });
      } else {
        showCommandError(result.error ?? t("commandFailed"));
      }
    }
    if (btn.dataset.expand) {
      const id = btn.dataset.expand;
      // Toggle by reading current DOM state to avoid importing module state.
      // NOTE: rows carry `data-provider-item`, not `data-provider`.
      const row = btn.closest("[data-provider-item]");
      const isOpen = row?.querySelector(".prov-detail") != null;
      setExpandedProvider(isOpen ? null : id);
      if (view === "settings") renderSettings(dash, settingsCategory);
    }
    if (btn.hasAttribute("data-detect")) {
      const ok = (await invokeCmd("detect_providers")).ok;
      if (!ok && isTauri()) showCommandError(t("commandFailed"));
    }
    if (btn.dataset.refreshProvider) {
      await invokeCmd("refresh_provider", { id: btn.dataset.refreshProvider });
    }
    if (btn.dataset.copyCli) {
      const copied = await copyToClipboard(btn.dataset.copyCli);
      if (copied) showToast(t("copiedCliCmd"));
    }
    if (btn.hasAttribute("data-copy-cli-path")) {
      const binaryPath = document.getElementById("cli-binary-path")?.textContent || "";
      if (binaryPath && binaryPath !== "—" && await copyToClipboard(binaryPath)) showToast(t("cliPathCopied"));
      return;
    }
    if (btn.hasAttribute("data-repair-cli")) {
      const result = await invokeCmd("repair_cli_path");
      if (result.ok) showToast(t("cliPathRepaired"));
      else showCommandError(result.error ?? t("commandFailed"));
      await reloadCliStatus();
      return;
    }
    if (btn.hasAttribute("data-test-cli")) {
      const result = await invokeCmd<string>("cli_test");
      const output = document.getElementById("cli-test-result");
      if (result.ok) {
        if (output) output.textContent = result.value;
        showToast(t("cliTestPassed"));
      } else {
        showCommandError(result.error ?? t("commandFailed"));
      }
      return;
    }
    // NOTE: <html> always carries data-theme (applyTheme sets it on load),
    // so an unscoped [data-theme] match would catch EVERY click via
    // closest() fallthrough and rebuild settings mid-gesture — killing
    // checkbox toggles, open selects and the action that was clicked.
    // Only real theme controls (never documentElement) may enter here.
    if (btn !== document.documentElement && btn.dataset.theme) {
      applyTheme(btn.dataset.theme as AppTheme);
      renderSettings(dash, settingsCategory);
    }
    if (btn.dataset.openUrl) await openExternal(btn.dataset.openUrl);
    if (btn.hasAttribute("data-install-update")) {
      const result = document.getElementById("update-result");
      const installButton = btn as HTMLButtonElement;
      installButton.disabled = true;
      if (result) {
        result.classList.remove("hidden");
        result.textContent = t("updateDownloading");
      }
      const installed = await invokeCmd("install_update");
      if (!installed.ok) {
        installButton.disabled = false;
        if (result) {
          result.textContent = installed.error ?? t("updateInstallFailed");
          result.classList.add("update-error");
        }
      } else if (result) {
        // El backend ya emitió `restarting`; este texto cubre el hueco
        // hasta que la app se cierre sola para dejar paso al instalador.
        result.textContent = t("updateRestarting");
      }
      return;
    }
    if (btn.hasAttribute("data-startpairing")) {
      await pairPhoneFlow();
      return;
    }
    if (btn.hasAttribute("data-revokedevice")) {
      const clientDeviceId = btn.getAttribute("data-revokedevice") ?? "";
      const result = await invokeCmd("sync_revoke_device", { clientDeviceId });
      if (!result.ok) showCommandError(result.error ?? t("commandFailed"));
      await refreshSyncView();
      return;
    }
    if (btn.hasAttribute("data-syncexport")) {
      const path = await invokeCmd<string>("sync_export_now");
      if (path.ok) showToast(`${t("syncExported")} ${path.value}`);
      else showCommandError(path.error ?? t("commandFailed"));
      if (view === "settings") await refreshSyncView();
    }
    if (btn.dataset.redeemReset && btn.dataset.resetUrl) {
      // The reset is owned by ChatGPT's Usage screen. Do not depend on a
      // private mutation endpoint; refresh the observed quota on return.
      refreshWhenFocused = btn.dataset.redeemReset;
      await openExternal(btn.dataset.resetUrl);
    }
  });

  document.addEventListener("toggle", (event) => {
    const section = event.target as HTMLDetailsElement;
    if (!section.matches("details[data-dashboard-section][data-provider-id]")) return;
    try {
      localStorage.setItem(
        `dashboard-section:${section.dataset.providerId}:${section.dataset.dashboardSection}`,
        section.open ? "1" : "0",
      );
    } catch {
      // Storage can be unavailable in restricted webviews; the default still works.
    }
  }, true);

  document.addEventListener("input", (e) => {
    // Credential drafts update on every keystroke into module state, so no
    // re-render can ever swallow a character. Nothing re-renders here, so
    // focus is never disturbed while typing.
    const el = e.target as HTMLElement;
    const keyInput = (el as HTMLInputElement).dataset?.key;
    if (keyInput && el instanceof HTMLInputElement) {
      setCredentialDraft(keyInput, el.value);
      // The eye follows the draft in place; the input node and focus survive.
      patchCredentialEye(keyInput, el.value);
    }
    if ((el as HTMLInputElement).dataset?.syncPass !== undefined && el instanceof HTMLInputElement) {
      setSyncPassphraseDraft(el.value);
    }
  });

  document.addEventListener("change", async (e) => {
    const el = e.target as HTMLElement;
    if ((el as HTMLSelectElement).dataset.source) {
      const input = el as HTMLSelectElement;
      const id = input.dataset.source!;
      const source = input.value;
      const vendor = dash?.catalog.find((item) => item.id === id);
      const previous = vendor?.sourcePreference ?? null;
      const preference = source === "auto" ? null : source;
      pendingSourcePreferences.set(id, preference);
      if (vendor) vendor.sourcePreference = preference;
      const ok = (await invokeCmd("set_source_preference", { id, source })).ok;
      pendingSourcePreferences.delete(id);
      if (!ok && isTauri()) {
        if (vendor) vendor.sourcePreference = previous;
        input.value = previous ?? "auto";
        showCommandError(t("commandFailed"));
      }
      return;
    }
    if ((el as HTMLInputElement).dataset.enable) {
      const input = el as HTMLInputElement;
      const id = input.dataset.enable!;
      const enabled = input.checked;
      // Optimistic local update: the new row (with an immediately editable
      // input) paints without waiting for the backend round-trip.
      const vendor = dash?.catalog.find((v) => v.id === id);
      pendingProviderEnabled.set(id, enabled);
      if (vendor) vendor.enabled = enabled;
      patchProviderInteractiveState(id, enabled);
      // Preserve the checkbox node that was just changed. The next dashboard
      // event patches status text only; it never replaces this form.
      const ok = (await invokeCmd("set_provider_enabled", { id, enabled })).ok;
      pendingProviderEnabled.delete(id);
      if (!ok && isTauri()) {
        // Backend rejected: revert model AND node so the UI never lies checked.
        if (vendor) vendor.enabled = !enabled;
        input.checked = !enabled;
        patchProviderInteractiveState(id, !enabled);
        showCommandError(t("commandFailed"));
      }
    }
    if (el.id === "cfg-sync-lan") {
      const lan = (el as HTMLInputElement).checked;
      const result = await invokeCmd("sync_set_lan", { lan });
      if (!result.ok && isTauri()) showCommandError(result.error ?? t("commandFailed"));
      if (view === "settings") await refreshSyncView();
    } else if (el.id === "cfg-autostart") {
      const enabled = (el as HTMLInputElement).checked;
      await invokeCmd("set_autostart_enabled", { enabled });
      if (dash) dash.autostart = enabled;
    } else if (el.id === "cfg-pin") {
      const enabled = (el as HTMLInputElement).checked;
      await invokeCmd("set_always_on_top", { enabled });
      if (dash) dash.alwaysOnTop = enabled;
    } else if (el.id === "cfg-notif") {
      const enabled = (el as HTMLInputElement).checked;
      await invokeCmd("set_notifications", { enabled });
      if (dash) dash.notifications = enabled;
    } else if (el.id === "cfg-compact") {
      const enabled = (el as HTMLInputElement).checked;
      if (dash) dash.compactMode = enabled;
      view = "dash";
      paintDash();
      await applyWindowMode(enabled);
      await invokeCmd("set_compact_mode", { enabled });
    } else if (el.id?.startsWith("cfg-")) {
      await persistConfig(dash);
    }
  });

  if (!isTauri()) {
    await applyDashboard(previewDashboard());
    return;
  }

  const { listen } = await import("@tauri-apps/api/event");
  await listen<Dashboard>("dashboard-updated", (ev) => applyDashboard(ev.payload));
  // Progreso real del updater: el backend emite la fase estable
  // (downloading/verifying/preparing/restarting) y aquí se mapea a i18n.
  // `downloadingUpdate` queda como fallback para payloads desconocidos.
  await listen<string>("updater-status", (ev) => {
    const messages: Record<string, string> = {
      downloading: t("updateDownloading"),
      verifying: t("updateVerifying"),
      preparing: t("updatePreparing"),
      restarting: t("updateRestarting"),
    };
    const result = document.getElementById("update-result");
    if (result && messages[ev.payload]) {
      result.classList.remove("hidden");
      result.textContent = messages[ev.payload];
    }
  });
  await listen<string>("tray-cmd", async (ev) => {
    // Payloads: "settings" | "settings:<category>" (from the tray "Manage
    // providers…" / "About" entries) | "pair-phone" (from "Vincular
    // teléfono"). This used to end with a call that re-ran the bare
    // "settings" case and reset settingsCategory back to "general" right
    // after the branch above had just applied the requested category —
    // removed, since it fought with the arg it just applied.
    const [cmd, arg] = (ev.payload || "").split(":");
    if (cmd === "settings") {
      const category = arg && ["general", "providers", "notifications", "appearance", "data", "sync", "about"].includes(arg)
        ? arg as SettingsCategory
        : "general";
      openSettingsCategory(category);
    } else if (cmd === "pair-phone") {
      await pairPhoneFlow();
    }
  });

  window.setInterval(updateResetClocks, 60_000);
  window.setInterval(updateLoadingClocks, 1_000);

  const d = await invokeCmd<Dashboard>("get_dashboard");
  if (d.ok) await applyDashboard(d.value);
  else renderBootstrapError();
}

main();
