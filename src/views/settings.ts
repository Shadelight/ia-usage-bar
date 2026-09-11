// Settings view: categorized nav (General/Proveedores/Notificaciones/
// Apariencia/Datos y registros/Acerca de) over the same underlying config.

import { $, escapeHtml, invokeCmd } from "../api";
import type { Dashboard, ProviderSnapshot, VendorInfo } from "../api";
import { lang, t } from "../i18n";
import { providerVisual } from "../providers";

export type SettingsCategory = "general" | "providers" | "notifications" | "appearance" | "data" | "about";

const CATEGORIES: SettingsCategory[] = ["general", "providers", "notifications", "appearance", "data", "about"];
const CATEGORY_LABEL: Record<SettingsCategory, string> = {
  general: "settingsGeneral",
  providers: "settingsProviders",
  notifications: "settingsNotifications",
  appearance: "settingsAppearance",
  data: "settingsData",
  about: "settingsAbout",
};

function logo(vendor: VendorInfo): string {
  const visual = providerVisual(vendor.id, vendor.name);
  return `<span class="provider-logo-wrap"><img class="provider-logo" src="${visual.icon}" alt="" />${visual.badge ? `<span class="provider-badge">${visual.badge}</span>` : ""}</span>`;
}

function checklistLabels(authKind: string): [string, string, string] {
  switch (authKind) {
    case "local": return [t("checklistApp"), t("checklistAccount"), t("checklistQuota")];
    case "oauth": return [t("checklistCli"), t("checklistSession"), t("checklistQuota")];
    case "apikey": return [t("checklistCredential"), t("checklistPermissions"), t("checklistQuota")];
    default: return [t("checklistApp"), t("checklistAccountOrCredential"), t("checklistQuota")];
  }
}

function checklistMarks(snapshot?: ProviderSnapshot): [string, string, string] {
  if (!snapshot) return ["—", "—", "—"];
  switch (snapshot.status) {
    case "connected": return ["✓", "✓", "✓"];
    case "needs_auth": return ["✓", "✕", "—"];
    case "needs_permission": return ["✓", "✓", "✕"];
    case "unavailable": return ["✕", "—", "—"];
    case "error": return ["✓", "✓", "✕"];
  }
}

function guide(vendor: VendorInfo, snapshot?: ProviderSnapshot): string {
  const labels = checklistLabels(vendor.authKind);
  const marks = checklistMarks(snapshot);
  return `<div class="guide-row">
    <strong>${escapeHtml(vendor.name)}</strong>
    <div class="connection-checklist">
      ${labels.map((label, index) => `<span><b class="check-${marks[index] === "✓" ? "ok" : marks[index] === "✕" ? "bad" : "idle"}">${marks[index]}</b>${escapeHtml(label)}</span>`).join("")}
    </div>
  </div>`;
}

function generalBody(dash: Dashboard): string {
  const enabled = dash.catalog.filter((v) => v.enabled);
  const primaryOpts = enabled
    .map((v) => `<option value="${escapeHtml(v.id)}" ${dash.primary === v.id ? "selected" : ""}>${escapeHtml(v.name)}</option>`)
    .join("");
  return `
    <label class="row">${t("autostart")}
      <input type="checkbox" id="cfg-autostart" ${dash.autostart ? "checked" : ""} />
    </label>
    <label class="row">${t("pin")}
      <input type="checkbox" id="cfg-pin" ${dash.alwaysOnTop ? "checked" : ""} />
    </label>
    <label class="row">${t("interval")}
      <select id="cfg-interval">
        <option value="1" ${dash.refreshMinutes === 1 ? "selected" : ""}>1 min</option>
        <option value="5" ${dash.refreshMinutes === 5 ? "selected" : ""}>5 min</option>
        <option value="10" ${dash.refreshMinutes === 10 ? "selected" : ""}>10 min</option>
      </select>
    </label>
    <label class="row">${t("primary")}
      <select id="cfg-primary">${primaryOpts}</select>
    </label>
  `;
}

function providersBody(dash: Dashboard): string {
  const cat = dash.catalog;
  const providers = cat
    .map((v) => {
      const snapshot = dash.providers.find((provider) => provider.id === v.id);
      const keyField = v.needsKey
        ? `<div class="key-row"><input data-key="${escapeHtml(v.id)}" placeholder="${escapeHtml(v.envKey || t("apiKey"))}" type="password" /><button data-savekey="${escapeHtml(v.id)}">${t("save")}</button></div>`
        : `<p class="hint">${escapeHtml(v.hint)}</p>`;
      const visual = providerVisual(v.id, v.name);
      return `<label class="prov" style="--provider-accent:${visual.accent}">
        <input type="checkbox" data-enable="${escapeHtml(v.id)}" ${v.enabled ? "checked" : ""} />
        ${logo(v)}
        <span class="prov-name">${escapeHtml(v.name)}</span>
        <span class="provider-status-dot status-${snapshot?.status || "loading"}"></span>
      </label>
      ${v.enabled ? keyField : ""}`;
    })
    .join("");
  return `
    <p class="lede">${t("providersHint")}</p>
    <div class="prov-list">${providers}</div>
    <h3>${t("guide")}</h3>
    <p class="lede">${t("guideHint")}</p>
    ${cat.map((v) => guide(v, dash.providers.find((provider) => provider.id === v.id))).join("")}
  `;
}

function notificationsBody(dash: Dashboard): string {
  return `
    <label class="row">${t("notifications")}
      <input type="checkbox" id="cfg-notif" ${dash.notifications ? "checked" : ""} />
    </label>
    <p class="lede">${t("notifHint")}</p>
    <label class="row">${t("notifyThresholds")}
      <input id="cfg-thresholds" inputmode="numeric" value="${dash.notifyThresholds.join(", ")}" aria-describedby="thresholds-hint" />
    </label>
    <p class="lede" id="thresholds-hint">${t("thresholdsHint")}</p>
  `;
}

function appearanceBody(dash: Dashboard): string {
  return `
    <label class="row">${t("compactMode")}
      <input type="checkbox" id="cfg-compact" ${dash.compactMode ? "checked" : ""} />
    </label>
    <label class="row">${t("theme")}
      <select id="cfg-theme" disabled><option>${t("themeSystem")}</option></select>
    </label>
    <label class="row">${t("language")}
      <span class="lang-seg">
        <button class="seg ${lang === "en" ? "active" : ""}" data-act="lang-en">EN</button>
        <button class="seg ${lang === "es" ? "active" : ""}" data-act="lang-es">ES</button>
      </span>
    </label>
  `;
}

function dataBody(): string {
  return `
    <button class="btn ghost" data-act="open-logs">${t("openLogs")}</button>
    <button class="btn ghost" data-act="clear-logs">${t("clearLogs")}</button>
    <button class="btn ghost" data-act="export-diagnostics">${t("exportDiagnostics")}</button>
  `;
}

const REPO_URL = "https://github.com/Shadelight/ia-usage-bar";

function aboutBody(): string {
  return `
    <h3>IA Usage</h3>
    <p class="lede" id="about-version">${t("version")} —</p>
    <p class="lede">${t("aboutTagline")}</p>
    <p class="lede">${t("aboutAuthor")}</p>
    <p class="lede"><a href="${REPO_URL}" target="_blank" rel="noreferrer">${t("aboutRepo")}</a></p>
    <p class="lede"><a href="${REPO_URL}/blob/main/LICENSE" target="_blank" rel="noreferrer">${t("aboutLicense")}</a></p>
    <p class="lede"><a href="${REPO_URL}/blob/main/NOTICE" target="_blank" rel="noreferrer">${t("aboutNotices")}</a></p>
    <p class="lede"><a href="${REPO_URL}/releases/latest" target="_blank" rel="noreferrer">${t("aboutCheckUpdates")}</a></p>
  `;
}

async function paintAboutVersion(): Promise<void> {
  const el = document.getElementById("about-version");
  if (!el) return;
  try {
    const { getVersion } = await import("@tauri-apps/api/app");
    el.textContent = `${t("version")} ${await getVersion()}`;
  } catch {
    /* browser preview / no Tauri: keep the placeholder */
  }
}

export function renderSettings(dash: Dashboard | null, category: SettingsCategory = "general"): void {
  if (!dash) return;
  $("settings-title").textContent = t("settings");
  $("settings-cats").innerHTML = CATEGORIES.map(
    (cat) => `<button class="settings-cat ${cat === category ? "active" : ""}" data-setcat="${cat}">${t(CATEGORY_LABEL[cat])}</button>`,
  ).join("");
  const body =
    category === "general" ? generalBody(dash)
    : category === "providers" ? providersBody(dash)
    : category === "notifications" ? notificationsBody(dash)
    : category === "appearance" ? appearanceBody(dash)
    : category === "data" ? dataBody()
    : aboutBody();
  $("settings-body").innerHTML = body;
  if (category === "about") void paintAboutVersion();
}

export async function persistConfig(dash: Dashboard | null): Promise<void> {
  if (!dash) return;
  const thresholdsInput = $("cfg-thresholds") as HTMLInputElement | null;
  const thresholds = (thresholdsInput?.value || "")
    .split(/[,;\s]+/)
    .map(Number)
    .filter((value) => Number.isInteger(value) && value >= 1 && value <= 99);
  await invokeCmd("set_app_config", {
    cfg: {
      refresh_minutes: Number((document.getElementById("cfg-interval") as HTMLSelectElement)?.value || dash.refreshMinutes),
      primary: (document.getElementById("cfg-primary") as HTMLSelectElement)?.value || dash.primary,
      notifications: (document.getElementById("cfg-notif") as HTMLInputElement)?.checked ?? dash.notifications,
      notify_thresholds: thresholds.length ? thresholds : dash.notifyThresholds,
      always_on_top: (document.getElementById("cfg-pin") as HTMLInputElement)?.checked ?? dash.alwaysOnTop,
      compact_mode: (document.getElementById("cfg-compact") as HTMLInputElement)?.checked ?? dash.compactMode,
      providers: Object.fromEntries(
        dash.catalog.map((v) => [
          v.id,
          { enabled: (document.querySelector(`[data-enable="${v.id}"]`) as HTMLInputElement)?.checked ?? v.enabled },
        ]),
      ),
    },
  });
}
