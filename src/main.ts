import "./styles.css";

import { $, escapeHtml, invokeCmd, isTauri } from "./api";
import type { Dashboard, ProviderStatus } from "./api";
import { sanitizeTechnicalDetails, shouldShowRecoveryToast } from "./errors";
import { setLang, t } from "./i18n";
import { renderDash, updateResetClocks } from "./views/dash";
import { persistConfig, renderSettings } from "./views/settings";
import type { SettingsCategory } from "./views/settings";
import { previewDashboard, renderSpend } from "./views/spend";

let dash: Dashboard | null = null;
let selectedId = localStorage.getItem("selected") || "";
let settingsCategory = (localStorage.getItem("settingsCategory") as SettingsCategory) || "general";
let view: "dash" | "spend" | "settings" = "dash";
const previousStatuses = new Map<string, ProviderStatus>();
let toastTimer: number | undefined;

function showRecoveryToast(name: string): void {
  const toast = $("status-toast");
  toast.textContent = t("recovered").replace("{name}", name);
  toast.classList.remove("hidden");
  window.clearTimeout(toastTimer);
  toastTimer = window.setTimeout(() => toast.classList.add("hidden"), 3_500);
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
    `<button data-act="compact">${t("compactMode")} ${dash?.compactMode ? "✓" : ""}</button>`,
    `<button data-act="toggle-notif">${t("notifications")} ${dash?.notifications ? "✓" : ""}</button>`,
    `<button data-act="spend">${t("spend")}</button>`,
    links?.usageUrl
      ? `<a href="${escapeHtml(links.usageUrl)}" target="_blank" rel="noreferrer">${t("providerPanel")}</a>`
      : "",
    links?.statusUrl
      ? `<a href="${escapeHtml(links.statusUrl)}" target="_blank" rel="noreferrer">${t("serviceStatus")}</a>`
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
  watchRecoveries(d);
  dash = d;
  if (view === "settings") {
    const draft = Object.fromEntries(
      [...document.querySelectorAll<HTMLInputElement>("input[data-key]")].map((i) => [
        i.dataset.key!,
        i.value,
      ]),
    );
    renderSettings(dash, settingsCategory);
    for (const [id, val] of Object.entries(draft)) {
      const el = document.querySelector<HTMLInputElement>(`[data-key="${id}"]`);
      if (el) el.value = val;
    }
  }
  if (view === "spend") renderSpend(dash);
  paintDash();
  await fitWindow();
}

async function fitWindow() {
  const head = document.querySelector(".head") as HTMLElement | null;
  const foot = document.querySelector(".foot") as HTMLElement | null;
  const viewEl =
    view === "settings" ? $("view-settings") : view === "spend" ? $("view-spend") : $("view-dash");
  const content =
    (head?.offsetHeight ?? 0) + (foot?.offsetHeight ?? 0) + viewEl.scrollHeight + 24;
  const compact = view === "dash" && !!dash?.compactMode;
  const w = compact ? 320 : 360;
  const h = compact ? Math.min(220, Math.max(160, content)) : Math.min(640, Math.max(420, content));
  if (!isTauri()) return;
  try {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    const { LogicalSize } = await import("@tauri-apps/api/dpi");
    await getCurrentWindow().setSize(new LogicalSize(w, h));
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
      await invokeCmd("refresh_now");
      break;
    case "detect":
      await invokeCmd("detect_providers");
      break;
    case "settings":
      view = "settings";
      renderSettings(dash, settingsCategory);
      paintDash();
      await fitWindow();
      break;
    case "spend":
      view = "spend";
      renderSpend(dash);
      paintDash();
      await fitWindow();
      break;
    case "back":
      view = "dash";
      paintDash();
      await fitWindow();
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
  }
}

async function main() {
  window.addEventListener("app-command-error", (event) => {
    showCommandError((event as CustomEvent<string>).detail || "unknown command error");
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
    const btn = target.closest<HTMLElement>("[data-act],[data-select],[data-savekey],[data-setcat]");
    if (!btn) {
      if (!target.closest("#head-menu, #btn-menu")) closeHeadMenu();
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
      await fitWindow();
    }
    if (btn.dataset.savekey) {
      const id = btn.dataset.savekey;
      const input = document.querySelector<HTMLInputElement>(`[data-key="${id}"]`);
      await invokeCmd("save_api_key", { id, key: input?.value || "" });
    }
  });

  document.addEventListener("change", async (e) => {
    const el = e.target as HTMLElement;
    if ((el as HTMLInputElement).dataset.enable) {
      const id = (el as HTMLInputElement).dataset.enable!;
      const enabled = (el as HTMLInputElement).checked;
      await invokeCmd("set_provider_enabled", { id, enabled });
    }
    if (el.id === "cfg-autostart") {
      const enabled = (el as HTMLInputElement).checked;
      await invokeCmd("set_autostart_enabled", { enabled });
      if (dash) dash.autostart = enabled;
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
    if (ev.payload === "settings") handleAction("settings");
  });

  window.setInterval(updateResetClocks, 60_000);

  const d = await invokeCmd<Dashboard>("get_dashboard");
  if (d) await applyDashboard(d);
  else renderBootstrapError();
}

main();
