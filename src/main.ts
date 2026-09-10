import "./styles.css";

import { $, invokeCmd, isTauri } from "./api";
import type { Dashboard } from "./api";
import { setLang } from "./i18n";
import { renderDash } from "./views/dash";
import { persistConfig, renderSettings } from "./views/settings";
import { previewDashboard, renderSpend } from "./views/spend";

let dash: Dashboard | null = null;
let selectedId = localStorage.getItem("selected") || "";
let view: "dash" | "spend" | "settings" = "dash";

function paintDash() {
  selectedId = renderDash(dash, selectedId);
  $("view-dash").classList.toggle("hidden", view !== "dash");
  $("view-spend").classList.toggle("hidden", view !== "spend");
  $("view-settings").classList.toggle("hidden", view !== "settings");
  $("btn-spend")?.classList.toggle("active", view === "spend");
}

async function applyDashboard(d: Dashboard) {
  dash = d;
  if (view === "settings") {
    const draft = Object.fromEntries(
      [...document.querySelectorAll<HTMLInputElement>("input[data-key]")].map((i) => [
        i.dataset.key!,
        i.value,
      ]),
    );
    renderSettings(dash);
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
  const h = Math.min(640, Math.max(420, content));
  if (!isTauri()) return;
  try {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    const { LogicalSize } = await import("@tauri-apps/api/dpi");
    await getCurrentWindow().setSize(new LogicalSize(360, h));
  } catch {
    /* ignore */
  }
}

async function handleAction(act: string) {
  switch (act) {
    case "refresh":
      await invokeCmd("refresh_now");
      break;
    case "detect":
      await invokeCmd("detect_providers");
      break;
    case "settings":
      view = "settings";
      renderSettings(dash);
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
      if (view === "settings") renderSettings(dash);
      if (view === "spend") renderSpend(dash);
      break;
    case "lang-es":
      setLang("es");
      paintDash();
      if (view === "settings") renderSettings(dash);
      if (view === "spend") renderSpend(dash);
      break;
  }
}

async function main() {
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
    const btn = (e.target as HTMLElement).closest<HTMLElement>("[data-act],[data-select],[data-savekey]");
    if (!btn) return;
    if (btn.dataset.act) await handleAction(btn.dataset.act);
    if (btn.dataset.select) {
      selectedId = btn.dataset.select;
      localStorage.setItem("selected", selectedId);
      paintDash();
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
    if (el.id?.startsWith("cfg-")) await persistConfig(dash);
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

  try {
    const d = await invokeCmd<Dashboard>("get_dashboard");
    if (d) await applyDashboard(d);
  } catch (err) {
    console.error(err);
  }
}

main();
