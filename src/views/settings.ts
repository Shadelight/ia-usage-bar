// Settings view: categorized nav (General/Proveedores/Notificaciones/
// Apariencia/Datos y registros/Acerca de) over the same underlying config.

import { $, escapeHtml, invokeCmd } from "../api";
import type { Dashboard, ProviderSnapshot, SyncPairingDto, SyncStatusDto, VendorInfo } from "../api";
import { lang, t } from "../i18n";
import { providerVisual } from "../providers";
import { actionIconSvg, getProviderActions } from "../provider-actions";
import { deriveProviderState, formatStatusText, isKeyInputDisabled } from "../provider-state";
import { activeItemScrollDelta, horizontalWheelDelta } from "../layout";

export type SettingsCategory = "general" | "providers" | "notifications" | "appearance" | "data" | "sync" | "about";

// Local UI state that must survive full body re-renders (renderSettings
// rebuilds innerHTML on every dashboard update).
let expandedProvider: string | null = null;
const credentialDrafts: Record<string, string> = {};
let savingProvider: string | null = null;

export function setExpandedProvider(id: string | null): void {
  expandedProvider = id;
}
export function setSavingProvider(id: string | null): void {
  savingProvider = id;
}
export function setCredentialDraft(id: string, value: string): void {
  credentialDrafts[id] = value;
}
export function clearCredentialDraft(id: string): void {
  delete credentialDrafts[id];
}

const CATEGORIES: SettingsCategory[] = ["general", "providers", "notifications", "appearance", "data", "sync", "about"];
const CATEGORY_LABEL: Record<SettingsCategory, string> = {
  general: "settingsGeneral",
  providers: "settingsProviders",
  notifications: "settingsNotifications",
  appearance: "settingsAppearance",
  data: "settingsData",
  sync: "settingsSync",
  about: "settingsAbout",
};

function logo(vendor: VendorInfo): string {
  const visual = providerVisual(vendor.id, vendor.name);
  return `<span class="provider-logo-wrap"><img class="provider-logo" src="${visual.icon}" alt="" />${visual.badge ? `<span class="provider-badge">${visual.badge}</span>` : ""}</span>`;
}

function toggleRow(id: string, label: string, checked: boolean): string {
  return `<label class="setting-toggle" for="${id}">
    <span>${escapeHtml(label)}</span>
    <span class="switch"><input type="checkbox" id="${id}" ${checked ? "checked" : ""} /><span class="switch-track" aria-hidden="true"><span></span></span></span>
  </label>`;
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
    ${toggleRow("cfg-autostart", t("autostart"), dash.autostart)}
    ${toggleRow("cfg-pin", t("pin"), dash.alwaysOnTop)}
    <label class="row">${t("interval")}
      <select id="cfg-interval">
        <option value="1" ${dash.refreshMinutes === 1 ? "selected" : ""}>1 min</option>
        <option value="2" ${dash.refreshMinutes === 2 ? "selected" : ""}>2 min</option>
        <option value="5" ${dash.refreshMinutes === 5 ? "selected" : ""}>5 min</option>
        <option value="15" ${dash.refreshMinutes === 15 ? "selected" : ""}>15 min</option>
        <option value="30" ${dash.refreshMinutes === 30 ? "selected" : ""}>30 min</option>
      </select>
    </label>
    ${toggleRow("cfg-adaptive", t("refreshAdaptive"), dash.refreshAdaptive)}
    <p class="lede">${t("adaptiveHint")}</p>
    <label class="row">${t("primary")}
      <select id="cfg-primary">${primaryOpts}</select>
    </label>
  `;
}

function providerLinks(vendor: VendorInfo): string {
  const items = getProviderActions(vendor).filter((a) => a.kind === "external" && a.url);
  if (!items.length) return "";
  return `<div class="prov-links">${items
    .map(
      (a) => `<button type="button" class="prov-link" data-open-url="${escapeHtml(a.url!)}">${actionIconSvg(a.icon, 12)}<span>${escapeHtml(t(a.labelKey))}</span></button>`,
    )
    .join("")}</div>`;
}

function providerKeyDetail(vendor: VendorInfo): string {
  const draft = credentialDrafts[vendor.id] ?? "";
  const busy = savingProvider === vendor.id;
  // Golden rule (see isKeyInputDisabled): editable as soon as enabled.
  const disabled = isKeyInputDisabled(vendor.enabled, busy);
  return `
    ${vendor.hasCredential ? `<p class="cred-state"><span aria-hidden="true">✓</span> ${escapeHtml(t("keyConfigured"))}</p>` : ""}
    <label class="key-label" for="key-${escapeHtml(vendor.id)}">${escapeHtml(t("apiKey"))}</label>
    <div class="key-row">
      <input id="key-${escapeHtml(vendor.id)}" data-key="${escapeHtml(vendor.id)}" type="password"
        placeholder="${escapeHtml(vendor.envKey || t("apiKey"))}" value="${escapeHtml(draft)}"
        autocomplete="off" spellcheck="false" ${disabled ? "disabled" : ""} />
      <button type="button" data-savekey="${escapeHtml(vendor.id)}" ${disabled ? "disabled" : ""}>${escapeHtml(busy ? t("saving") : t("saveCredential"))}</button>
      ${vendor.hasCredential ? `<button type="button" class="ghost" data-delkey="${escapeHtml(vendor.id)}" ${busy ? "disabled" : ""}>${escapeHtml(t("deleteCredential"))}</button>` : ""}
    </div>`;
}

function providerLoginDetail(vendor: VendorInfo): string {
  const cliCmd = (vendor.hint.match(/`([^`]+)`/) || [])[1] || "";
  return `
    <p class="login-hint">${escapeHtml(vendor.hint)}</p>
    <div class="prov-buttons">
      ${cliCmd ? `<button type="button" data-copy-cli="${escapeHtml(cliCmd)}">${actionIconSvg("copy", 12)}<span>${escapeHtml(t("copyCliCmd"))}</span></button>` : ""}
      <button type="button" data-detect>${actionIconSvg("refresh", 12)}<span>${escapeHtml(t("detectAgain"))}</span></button>
    </div>`;
}

function strategyLabel(slug: string): string {
  return slug === "oauth" ? "OAuth" : slug.charAt(0).toUpperCase() + slug.slice(1);
}

function sourceName(source: string | null | undefined): string {
  if (!source) return "—";
  const s = source.toLowerCase().replace("local-session", "local").replace("web-session", "web");
  return strategyLabel(s);
}

function providerSourceDetail(vendor: VendorInfo, snapshot?: ProviderSnapshot): string {
  const strategies = vendor.strategies?.length ? vendor.strategies : ["auto"];
  const current = vendor.sourcePreference ?? "auto";
  const opts = [`<option value="auto" ${current === "auto" ? "selected" : ""}>${escapeHtml(t("autoSource"))}</option>`]
    .concat(strategies.filter((s) => s !== "auto").map(
      (s) => `<option value="${escapeHtml(s)}" ${current === s ? "selected" : ""}>${escapeHtml(strategyLabel(s))}</option>`,
    ))
    .join("");
  const active = snapshot?.activeSource ? `<p class="lede">${escapeHtml(t("usingNow"))}: <strong>${escapeHtml(sourceName(snapshot.activeSource))}</strong></p>` : "";
  if (strategies.length <= 1 && !snapshot?.activeSource) return "";
  return `<label class="row">${escapeHtml(t("sourceLabel"))}
      <select data-source="${escapeHtml(vendor.id)}">${opts}</select>
    </label>${active}`;
}

function providerDetail(vendor: VendorInfo, snapshot?: ProviderSnapshot): string {
  const showKey = vendor.needsKey;
  const showLogin = vendor.authKind === "oauth" || vendor.authKind === "local" || vendor.authKind === "mixed";
  return `<div class="prov-detail" id="prov-detail-${escapeHtml(vendor.id)}" role="region" aria-label="${escapeHtml(vendor.name)}">
    ${providerSourceDetail(vendor, snapshot)}
    ${showKey ? providerKeyDetail(vendor) : ""}
    ${showLogin ? providerLoginDetail(vendor) : ""}
    <div class="prov-buttons">
      <button type="button" data-refresh-provider="${escapeHtml(vendor.id)}">${actionIconSvg("refresh", 12)}<span>${escapeHtml(t("actionRetry"))}</span></button>
      ${vendor.links.appUrl ? `<button type="button" data-open-url="${escapeHtml(vendor.links.appUrl)}">${actionIconSvg("app", 12)}<span>${escapeHtml(t("openApp"))}</span></button>` : ""}
    </div>
    ${providerLinks(vendor)}
  </div>`;
}

function providerRow(vendor: VendorInfo, snapshot: ProviderSnapshot | undefined, loading: boolean): string {
  const d = deriveProviderState(vendor, snapshot, loading);
  const expanded = expandedProvider === vendor.id;
  const visual = providerVisual(vendor.id, vendor.name);
  const expandLabel = expanded ? t("collapseProvider") : t("expandProvider");
  return `<div class="prov-row" id="prov-config-${escapeHtml(vendor.id)}" data-provider-item="${escapeHtml(vendor.id)}" style="--provider-accent:${visual.accent}">
    <div class="prov-head">
      <input type="checkbox" class="ds-check" data-enable="${escapeHtml(vendor.id)}" id="prov-check-${escapeHtml(vendor.id)}"
        aria-label="${escapeHtml(t("enableProvider").replace("{name}", vendor.name))}" ${vendor.enabled ? "checked" : ""} />
      ${logo(vendor)}
      <button type="button" class="prov-main" data-expand="${escapeHtml(vendor.id)}" aria-expanded="${expanded ? "true" : "false"}"
        aria-controls="prov-detail-${escapeHtml(vendor.id)}" aria-label="${escapeHtml(vendor.name)} — ${escapeHtml(expandLabel)}">
        <span class="prov-name">${escapeHtml(vendor.name)}</span>
        <span class="prov-sub">${escapeHtml(formatStatusText(d))}</span>
      </button>
      <span class="provider-status-dot ${d.dotClass}" aria-hidden="true"></span>
      <span class="prov-chev${expanded ? " open" : ""}" aria-hidden="true">${actionIconSvg("chevron-right", 14)}</span>
    </div>
    ${expanded ? providerDetail(vendor, snapshot) : ""}
  </div>`;
}

function providersBody(dash: Dashboard): string {
  const rows = dash.catalog
    .map((v) => providerRow(v, dash.providers.find((p) => p.id === v.id), dash.loadingProviders.includes(v.id)))
    .join("");
  return `
    <p class="lede">${t("providersLede")}</p>
    <div class="prov-list">${rows}</div>
    <h3>${t("guide")}</h3>
    <p class="lede">${t("guideHint")}</p>
    ${dash.catalog.map((v) => guide(v, dash.providers.find((provider) => provider.id === v.id))).join("")}
  `;
}

function notificationsBody(dash: Dashboard): string {
  return `
    ${toggleRow("cfg-notif", t("notifications"), dash.notifications)}
    <p class="lede">${t("notifHint")}</p>
    <label class="row">${t("notifyThresholds")}
      <input id="cfg-thresholds" inputmode="numeric" value="${dash.notifyThresholds.join(", ")}" aria-describedby="thresholds-hint" />
    </label>
    <p class="lede" id="thresholds-hint">${t("thresholdsHint")}</p>
  `;
}

function appearanceBody(dash: Dashboard): string {
  const theme = localStorage.getItem("theme") || "system";
  return `
    ${toggleRow("cfg-compact", t("compactMode"), dash.compactMode)}
    <div class="setting-stack"><span>${t("theme")}</span>
      <span class="choice-seg" role="group" aria-label="${t("theme")}">
        <button class="seg ${theme === "system" ? "active" : ""}" data-theme="system">${t("themeSystem")}</button>
        <button class="seg ${theme === "light" ? "active" : ""}" data-theme="light">${t("themeLight")}</button>
        <button class="seg ${theme === "dark" ? "active" : ""}" data-theme="dark">${t("themeDark")}</button>
      </span>
    </div>
    <div class="setting-stack"><span>${t("language")}</span>
      <span class="choice-seg" role="group" aria-label="${t("language")}">
        <button class="seg ${lang === "es" ? "active" : ""}" data-act="lang-es">Español</button>
        <button class="seg ${lang === "en" ? "active" : ""}" data-act="lang-en">English</button>
      </span>
    </div>
  `;
}

function dataBody(): string {
  return `
    <button class="btn ghost" data-act="open-logs">${t("openLogs")}</button>
    <button class="btn ghost" data-act="clear-logs">${t("clearLogs")}</button>
    <button class="btn ghost" data-act="export-diagnostics">${t("exportDiagnostics")}</button>
  `;
}

// --- M5 sync con el teléfono (categoría "sync") ---

let syncCategory: SettingsCategory = "general";
let syncStatus: SyncStatusDto | null = null;
let syncPairing: SyncPairingDto | null = null;
let syncPassphraseDraft = "";

export function setSyncPassphraseDraft(value: string): void {
  syncPassphraseDraft = value;
}

export function clearSyncPassphraseDraft(): void {
  syncPassphraseDraft = "";
}

export function setSyncPairing(dto: SyncPairingDto | null): void {
  syncPairing = dto;
}

function syncStatusRow(label: string, value: string, id: string): string {
  return `<div class="row"><span>${escapeHtml(label)}</span><strong id="${id}">${escapeHtml(value)}</strong></div>`;
}

function syncBody(): string {
  const st = syncStatus;
  const server = st
    ? `${st.serverRunning ? t("syncServerRunning") : t("syncServerStopped")}${st.serverRunning ? ` · ${st.serverAddr}` : ""}`
    : "—";
  const lastExport = st?.lastExport ? `${st.lastExport.path} (${st.lastExport.bytes} B)` : t("syncNeverExported");
  return `
    <p class="lede">${t("syncLede")}</p>
    ${toggleRow("cfg-sync", t("syncEnable"), st?.enabled ?? false)}
    ${syncStatusRow(t("syncDevice"), st ? `${st.deviceId} (${st.fingerprint})` : "—", "sync-device")}
    ${syncStatusRow(t("syncFolder"), st?.exportDir ?? "—", "sync-dir")}
    ${syncStatusRow(t("syncServer"), server, "sync-server")}
    ${syncStatusRow(t("syncLastExport"), lastExport, "sync-export")}
    <label class="key-label" for="sync-pass">${escapeHtml(t("syncPassphrase"))}</label>
    <div class="key-row">
      <input id="sync-pass" data-sync-pass type="password"
        placeholder="${escapeHtml(t("syncPassHint"))}" value="${escapeHtml(syncPassphraseDraft)}"
        autocomplete="new-password" spellcheck="false" />
      <button type="button" data-savesyncpass>${escapeHtml(t("syncSavePassphrase"))}</button>
      ${st?.hasPassphrase ? `<button type="button" class="ghost" data-forgetsyncpass>${escapeHtml(t("syncForgetPassphrase"))}</button>` : ""}
    </div>
    ${toggleRow("cfg-sync-lan", t("syncLanExpose"), st?.lan ?? false)}
    <p class="lede">${t("syncLanHint")}</p>
    <div class="prov-buttons">
      <button type="button" data-syncqr>${actionIconSvg("external-link", 12)}<span>${escapeHtml(t("syncShowQr"))}</span></button>
      <button type="button" data-syncexport>${actionIconSvg("refresh", 12)}<span>${escapeHtml(t("syncExportNow"))}</span></button>
    </div>
    <div id="sync-qr" class="sync-qr">${syncPairing ? `
      <img src="data:image/png;base64,${syncPairing.qrPngBase64}" alt="QR" />
      <p class="sync-uri">${escapeHtml(syncPairing.uri)}</p>
      <p class="lede">${escapeHtml(t("syncFingerprint"))}: <strong>${escapeHtml(syncPairing.fingerprint)}</strong></p>
    ` : ""}</div>
  `;
}

/** Trae el estado y parchea los nodos (nunca rebuild: no roba el foco). */
export async function reloadSyncStatus(): Promise<void> {
  const st = await invokeCmd<SyncStatusDto>("sync_get_status");
  if (!st) return;
  syncStatus = st;
  const set = (id: string, value: string): void => {
    const el = document.getElementById(id);
    if (el) el.textContent = value;
  };
  set("sync-device", `${st.deviceId} (${st.fingerprint})`);
  set("sync-dir", st.exportDir);
  set(
    "sync-server",
    `${st.serverRunning ? t("syncServerRunning") : t("syncServerStopped")}${st.serverRunning ? ` · ${st.serverAddr}` : ""}`,
  );
  set("sync-export", st.lastExport ? `${st.lastExport.path} (${st.lastExport.bytes} B)` : t("syncNeverExported"));
  const toggle = document.getElementById("cfg-sync") as HTMLInputElement | null;
  if (toggle && document.activeElement !== toggle) toggle.checked = st.enabled;
  const lan = document.getElementById("cfg-sync-lan") as HTMLInputElement | null;
  if (lan && document.activeElement !== lan) lan.checked = st.lan;
}

/**
 * Re-render de la categoría sync tras una acción explícita. Si el usuario
 * está escribiendo la passphrase, solo parchea estado.
 */
export function refreshSyncView(): void {
  if (syncCategory !== "sync") return;
  const typing = (document.activeElement as HTMLElement | null)?.id === "sync-pass";
  if (typing && syncPassphraseDraft) {
    void reloadSyncStatus();
    return;
  }
  const body = document.getElementById("settings-body");
  if (body) body.innerHTML = syncBody();
  void reloadSyncStatus();
}

const REPO_URL = "https://github.com/Shadelight/ia-usage-bar";

function aboutBody(): string {
  const action = (label: string, url: string) => `<button class="about-action" data-open-url="${url}"><span class="about-action-icon" aria-hidden="true">${actionIconSvg("external-link", 14)}</span><span>${escapeHtml(label)}</span><span class="about-action-arrow" aria-hidden="true">${actionIconSvg("chevron-right", 14)}</span></button>`;
  return `
    <section class="about-hero"><span class="about-mark">IA</span><div><h3>IA Usage</h3><p id="about-version">${t("version")} —</p></div></section>
    <p class="about-copy">${t("aboutTagline")}</p>
    <p class="about-author">${t("aboutAuthor")}</p>
    <div class="about-actions">
      ${action(t("aboutRepo"), REPO_URL)}
      ${action(t("aboutLicense"), `${REPO_URL}/blob/main/LICENSE`)}
      ${action(t("aboutNotices"), `${REPO_URL}/blob/main/NOTICE`)}
      <button class="about-action" data-act="check-updates"><span class="about-action-icon" aria-hidden="true">${actionIconSvg("refresh", 14)}</span><span>${t("aboutCheckUpdates")}</span><span class="about-action-arrow" aria-hidden="true">${actionIconSvg("chevron-right", 14)}</span></button>
    </div>
    <div id="update-result" class="update-result hidden" role="status"></div>
  `;
}

export async function checkForUpdates(): Promise<void> {
  const result = document.getElementById("update-result");
  if (!result) return;
  result.classList.remove("hidden", "update-available", "update-error");
  result.textContent = t("checkingUpdates");
  const update = await invokeCmd<{ current: string; latest: string; url: string; updateAvailable: boolean }>("check_for_updates");
  if (!update) {
    result.textContent = t("updateCheckFailed");
    result.classList.add("update-error");
    return;
  }
  if (update.updateAvailable) {
    result.innerHTML = `<span>${t("updateAvailable")} · ${escapeHtml(update.latest)}</span><button data-open-url="${escapeHtml(update.url)}">${t("viewRelease")}</button>`;
    result.classList.add("update-available");
  } else {
    result.textContent = `${t("upToDate")} · IA Usage ${escapeHtml(update.current)}`;
  }
}

function wireCategoryScroll(revealActive: boolean, previousScroll: number): void {
  const nav = $("settings-cats");
  nav.onwheel = (event) => {
    if (nav.scrollWidth <= nav.clientWidth) return;
    event.preventDefault();
    nav.scrollLeft += horizontalWheelDelta(event.deltaX, event.deltaY);
  };
  requestAnimationFrame(() => {
    if (!revealActive) {
      nav.scrollLeft = previousScroll;
      return;
    }
    const active = nav.querySelector<HTMLElement>(".active");
    if (!active) return;
    const delta = activeItemScrollDelta(nav.getBoundingClientRect(), active.getBoundingClientRect());
    if (delta) nav.scrollBy({ left: delta, behavior: "smooth" });
  });
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
  syncCategory = category;
  const previousCategory = $("settings-cats").querySelector<HTMLElement>(".settings-cat.active")?.dataset.setcat;
  const previousCategoryScroll = $("settings-cats").scrollLeft;
  const previousBodyScroll = $("settings-body").scrollTop;
  const categoryChanged = previousCategory !== category;
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
    : category === "sync" ? syncBody()
    : aboutBody();
  $("settings-body").innerHTML = body;
  $("settings-body").scrollTop = categoryChanged ? 0 : previousBodyScroll;
  wireCategoryScroll(categoryChanged, previousCategoryScroll);
  if (category === "about") void paintAboutVersion();
  if (category === "sync") void reloadSyncStatus();
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
      refresh_adaptive: (document.getElementById("cfg-adaptive") as HTMLInputElement)?.checked ?? dash.refreshAdaptive,
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

export function focusProviderInSettings(providerId: string): void {
  requestAnimationFrame(() => {
    const el = document.getElementById(`prov-config-${providerId}`);
    if (el) {
      el.scrollIntoView({ behavior: "smooth", block: "center" });
      el.classList.remove("prov-highlight");
      void el.offsetWidth;
      el.classList.add("prov-highlight");
      window.setTimeout(() => el.classList.remove("prov-highlight"), 2000);
      const input = el.querySelector<HTMLInputElement>("input[data-key]");
      if (input) input.focus();
    }
  });
}
