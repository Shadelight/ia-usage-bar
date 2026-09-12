import "./styles.css";

import { $, escapeHtml, invokeCmd, isTauri } from "./api";
import type { Dashboard, ProviderStatus, SyncPairingDto } from "./api";
import { sanitizeTechnicalDetails, shouldShowRecoveryToast } from "./errors";
import { setLang, t } from "./i18n";
import { buildSanitizedDiagnosis, renderDash, updateLoadingClocks, updateResetClocks } from "./views/dash";
import { actionIconSvg } from "./provider-actions";
import { checkForUpdates, clearCredentialDraft, clearSyncPassphraseDraft, focusProviderInSettings, patchSettings, persistConfig, refreshSyncView, renderSettings, setCredentialDraft, setExpandedProvider, setSavingProvider, setSyncPairing, setSyncPassphraseDraft } from "./views/settings";
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
  toast.textContent = t("commandFailed");
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

function paintHeadMenu() {
  $("btn-pin").classList.toggle("active", !!dash?.alwaysOnTop);
  const vendor = dash?.catalog.find((v) => v.id === selectedId);
  const links = vendor?.links;
  const rows = [
    `<button data-act="compact"><span>${t("compactMode")}</span>${dash?.compactMode ? `<span class="menu-check">${actionIconSvg("check", 13)}</span>` : ""}</button>`,
    `<button data-act="toggle-notif"><span>${t("notifications")}</span>${dash?.notifications ? `<span class="menu-check">${actionIconSvg("check", 13)}</span>` : ""}</button>`,
    `<button data-act="spend">${t("spend")}</button>`,
    links?.usageUrl
      ? `<button data-open-url="${escapeHtml(links.usageUrl)}">${t("providerPanel")}</button>`
      : "",
    links?.statusUrl
      ? `<button data-open-url="${escapeHtml(links.statusUrl)}">${t("serviceStatus")}</button>`
      : "",
    `<button data-act="open-logs">${t("openLogs")}</button>`,
    `<button data-act="settings">${t("settings")}</button>`,
  ].filter(Boolean);
  $("head-menu").innerHTML = rows.join("");
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
    const enabled = pendingProviderEnabled.get(vendor.id);
    if (enabled !== undefined) vendor.enabled = enabled;
    if (pendingSourcePreferences.has(vendor.id)) {
      vendor.sourcePreference = pendingSourcePreferences.get(vendor.id) ?? null;
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
      const enabled = !dash?.notifications;
      await invokeCmd("set_notifications", { enabled });
      if (dash) dash.notifications = enabled;
      paintHeadMenu();
      break;
    }
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
      view = "settings";
      renderSettings(dash, settingsCategory);
      paintDash();
      break;
    case "manage-providers":
      // The provider catalog is intentionally curated by VendorId::all();
      // “Add” means enable one of those supported integrations, rather than
      // pretending that an arbitrary custom provider can be queried.
      view = "settings";
      settingsCategory = "providers";
      localStorage.setItem("settingsCategory", settingsCategory);
      renderSettings(dash, settingsCategory);
      paintDash();
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
    const btn = target.closest<HTMLElement>("[data-act],[data-select],[data-savekey],[data-delkey],[data-expand],[data-detect],[data-refresh-provider],[data-copy-cli],[data-setcat],[data-theme],[data-open-url],[data-redeem-reset],[data-savesyncpass],[data-forgetsyncpass],[data-syncqr],[data-syncexport]");
    if (!btn) {
      if (!target.closest("#head-menu, #btn-menu")) closeHeadMenu();
      return;
    }
    if (btn.dataset.act === "configure-provider") {
      const providerId = btn.dataset.providerId || selectedId;
      view = "settings";
      settingsCategory = "providers";
      localStorage.setItem("settingsCategory", settingsCategory);
      setExpandedProvider(providerId || null);
      renderSettings(dash, settingsCategory);
      paintDash();
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
      settingsCategory = btn.dataset.setcat as SettingsCategory;
      localStorage.setItem("settingsCategory", settingsCategory);
      renderSettings(dash, settingsCategory);
    }
    if (btn.dataset.savekey) {
      const id = btn.dataset.savekey;
      const input = document.querySelector<HTMLInputElement>(`[data-key="${id}"]`);
      setCredentialDraft(id, input?.value || "");
      setSavingProvider(id);
      renderSettings(dash, settingsCategory);
      const saved = (await invokeCmd("save_api_key", { id, key: input?.value || "" })).ok;
      setSavingProvider(null);
      // On success the secret stays server-side only: drop the draft so the
      // input renders empty with a "saved" badge. On failure keep the draft
      // (what the user typed is never discarded without confirmation).
      if (saved || !isTauri()) clearCredentialDraft(id);
      else showCommandError(t("commandFailed"));
      if (view === "settings") renderSettings(dash, settingsCategory);
    }
    if (btn.dataset.delkey) {
      const id = btn.dataset.delkey;
      const deleted = (await invokeCmd("delete_api_key", { id })).ok;
      if (deleted || !isTauri()) {
        clearCredentialDraft(id);
        if (view === "settings") renderSettings(dash, settingsCategory);
      } else {
        showCommandError(t("commandFailed"));
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
    if (btn.hasAttribute("data-savesyncpass")) {
      const input = document.getElementById("sync-pass") as HTMLInputElement | null;
      const passphrase = input?.value || "";
      const saved = (await invokeCmd("sync_set_passphrase", { passphrase })).ok;
      if (saved || !isTauri()) {
        clearSyncPassphraseDraft();
        setSyncPairing(null);
        showToast(t("syncPassphraseSaved"));
      } else {
        showCommandError(t("commandFailed"));
      }
      if (view === "settings") refreshSyncView();
    }
    if (btn.hasAttribute("data-forgetsyncpass")) {
      const done = (await invokeCmd("sync_set_passphrase", { passphrase: "" })).ok;
      if (done || !isTauri()) {
        clearSyncPassphraseDraft();
        setSyncPairing(null);
        showToast(t("syncUpdated"));
      } else {
        showCommandError(t("commandFailed"));
      }
      if (view === "settings") refreshSyncView();
    }
    if (btn.hasAttribute("data-syncqr")) {
      const lanToggle = document.getElementById("cfg-sync-lan") as HTMLInputElement | null;
      const pairing = await invokeCmd<SyncPairingDto>("sync_get_pairing", { lan: lanToggle?.checked ?? false });
      if (pairing.ok) setSyncPairing(pairing.value);
      else showCommandError(t("commandFailed"));
      if (view === "settings") refreshSyncView();
    }
    if (btn.hasAttribute("data-syncexport")) {
      const path = await invokeCmd<string>("sync_export_now");
      if (path.ok) showToast(`${t("syncExported")} ${path.value}`);
      else showCommandError(t("commandFailed"));
      if (view === "settings") refreshSyncView();
    }
    if (btn.dataset.redeemReset && btn.dataset.resetUrl) {
      // The reset is owned by ChatGPT's Usage screen. Do not depend on a
      // private mutation endpoint; refresh the observed quota on return.
      refreshWhenFocused = btn.dataset.redeemReset;
      await openExternal(btn.dataset.resetUrl);
    }
  });

  document.addEventListener("input", (e) => {
    // Credential drafts update on every keystroke into module state, so no
    // re-render can ever swallow a character. Nothing re-renders here, so
    // focus is never disturbed while typing.
    const el = e.target as HTMLElement;
    const keyInput = (el as HTMLInputElement).dataset?.key;
    if (keyInput && el instanceof HTMLInputElement) setCredentialDraft(keyInput, el.value);
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
      // Preserve the checkbox node that was just changed. The next dashboard
      // event patches status text only; it never replaces this form.
      const ok = (await invokeCmd("set_provider_enabled", { id, enabled })).ok;
      pendingProviderEnabled.delete(id);
      if (!ok && isTauri()) {
        // Backend rejected: revert model AND node so the UI never lies checked.
        if (vendor) vendor.enabled = !enabled;
        input.checked = !enabled;
        showCommandError(t("commandFailed"));
      }
    }
    if (el.id === "cfg-sync") {
      const enabled = (el as HTMLInputElement).checked;
      const ok = (await invokeCmd("sync_set_enabled", { enabled })).ok;
      if (!ok && isTauri()) {
        showCommandError(t("syncNeedPassphrase"));
      }
      if (view === "settings") refreshSyncView();
    } else if (el.id === "cfg-sync-lan") {
      const lan = (el as HTMLInputElement).checked;
      const ok = (await invokeCmd("sync_set_lan", { lan })).ok;
      if (!ok && isTauri()) showCommandError(t("commandFailed"));
      if (view === "settings") refreshSyncView();
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
  await listen<string>("tray-cmd", (ev) => {
    // Payloads: "settings" | "settings:<category>" (e.g. from the tray
    // "Manage providers…" / "About" entries).
    const [cmd, arg] = (ev.payload || "").split(":");
    if (cmd === "settings") {
      view = "settings";
      if (arg === "providers" || arg === "about" || arg === "general" || arg === "data" || arg === "sync") {
        settingsCategory = arg as SettingsCategory;
        localStorage.setItem("settingsCategory", settingsCategory);
      }
      renderSettings(dash, settingsCategory);
      paintDash();
      handleAction("settings");
    }
  });

  window.setInterval(updateResetClocks, 60_000);
  window.setInterval(updateLoadingClocks, 1_000);

  const d = await invokeCmd<Dashboard>("get_dashboard");
  if (d.ok) await applyDashboard(d.value);
  else renderBootstrapError();
}

main();
