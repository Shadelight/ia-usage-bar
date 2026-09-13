// Main dashboard view: provider tabs + the selected provider's detail.

import { $, escapeHtml } from "../api.ts";
import type { Dashboard, MetricLine, ProviderSnapshot, UsageQuota, VendorInfo } from "../api.ts";
import { normalizeProviderError } from "../errors.ts";
import { lang, t } from "../i18n.ts";
import { deriveProviderState, formatStatusText } from "../provider-state.ts";
import { providerVisual } from "../providers.ts";
import { formatResetAbsolute, formatResetRelative } from "../reset-format.ts";
import { activeItemScrollDelta, elapsedLabel, horizontalWheelDelta } from "../layout.ts";
import {
  actionIconSvg,
  buildSanitizedDiagnosis,
  getCompactQuickActions,
  getProviderActions,
  getProviderCliCommand,
  pctOf,
} from "../provider-actions.ts";
import { tabHtml } from "../provider-tabs.ts";
import { recCopy, recPrefix, recWhyTitle } from "../recommend.ts";

export { actionIconSvg, buildSanitizedDiagnosis };

const loadingStarted = new Map<string, number>();
let lastRevealedProvider = "";

function windowSeconds(quota: UsageQuota): number {
  switch (quota.windowType) {
    case "session":
    case "5h": return 5 * 3_600;
    case "daily": return 86_400;
    case "weekly": return 7 * 86_400;
    case "monthly": return 30 * 86_400;
    default: return 0;
  }
}

interface PaceInfo {
  expected: number;
  actual: number;
  delta: number;
  willLast: boolean | null;
  exhaustAt: number | null;
}

/** Misma fórmula que `iausage-core::pace::compute_pace`. */
function paceInfo(quota: UsageQuota, now = Date.now()): PaceInfo | null {
  if (!quota.resetAt) return null;
  const duration = windowSeconds(quota);
  if (duration <= 0) return null;
  const end = new Date(quota.resetAt).getTime();
  if (!Number.isFinite(end)) return null;
  const elapsed = Math.max(0, duration - (end - now) / 1_000);
  const fraction = Math.max(0, Math.min(1, elapsed / duration));
  const utilization = pctOf(quota);
  if (fraction < 0.05 || utilization == null || utilization < 8) return null;
  const expected = fraction * 100;
  const delta = utilization - expected;
  const rate = utilization / Math.max(elapsed, 1);
  if (rate <= 0) return { expected, actual: utilization, delta, willLast: null, exhaustAt: null };
  const exhaustAt = now + ((100 - utilization) / rate) * 1_000;
  return { expected, actual: utilization, delta, willLast: exhaustAt >= end, exhaustAt };
}

function durLabel(ms: number): string {
  const hours = Math.floor(ms / 3_600_000);
  const minutes = Math.round((ms % 3_600_000) / 60_000);
  return `${hours}h ${minutes}m`;
}

function paceHtml(quota: UsageQuota): string {
  const p = paceInfo(quota);
  if (!p || Math.abs(p.delta) < 5) return "";
  const pts = Math.round(Math.abs(p.delta));
  if (p.delta > 0 && p.willLast === false && quota.resetAt) {
    const before = new Date(quota.resetAt).getTime() - (p.exhaustAt ?? Date.now());
    const detail = before > 0 ? ` · ${t("paceExhausts").replace("{dur}", durLabel(before))}` : "";
    return `<div class="pace">⚠ ${escapeHtml(t("paceOver").replace("{delta}", String(pts)))}${escapeHtml(detail)}</div>`;
  }
  if (p.delta > 0) {
    return `<div class="pace">⚠ ${escapeHtml(t("paceOver").replace("{delta}", String(pts)))}</div>`;
  }
  return `<div class="pace pace-ok">ℹ ${escapeHtml(t("paceUnder").replace("{delta}", String(pts)))}</div>`;
}

function resetState(quota: UsageQuota): string {
  switch (quota.resetStatus) {
    case "not_provided": return t("resetNotProvided");
    case "not_applicable": return t("resetNotApplicable");
    case "fetch_failed": return t("resetFetchFailed");
    case "known": return quota.resetAt ? "" : t("resetFetchFailed");
  }
}

function temporaryLimit(quota: UsageQuota): string {
  if (quota.temporaryMultiplier == null || quota.temporaryMultiplier <= 1) return "";
  const increase = Math.round((quota.temporaryMultiplier - 1) * 100);
  const expires = formatResetAbsolute(quota.temporaryExpiresAt, lang);
  const suffix = expires ? ` ${t("until")} ${expires}` : "";
  return `<div class="temporary-limit"><strong>${t("temporaryLimit")}</strong> +${increase}%${escapeHtml(suffix)}</div>`;
}

function progressBlock(quota: UsageQuota): string {
  const percent = pctOf(quota);
  const remaining = quota.remainingPercent == null ? null : Math.max(0, Math.min(100, quota.remainingPercent));
  const pace = paceHtml(quota);
  const resetAt = quota.resetAt ? escapeHtml(quota.resetAt) : "";
  const knownReset = quota.resetStatus === "known" && !!quota.resetAt;
  if (import.meta.env.DEV) console.debug(`[usage rendered ${quota.id}]`, quota);
  return `<article class="block">
    <div class="block-top">
      <div>
        <div class="kicker">${escapeHtml(quota.label)}</div>
        <div class="pct-row">
          <span class="pct">${percent == null ? "—" : `${Math.round(percent)}%`}</span>
          <span class="used-word">${t("used")}</span>
        </div>
        <div class="remain">${remaining == null ? t("notAvailable") : `${Math.round(remaining)}% ${t("available")}`}</div>
      </div>
      <div class="reset-col ${knownReset ? "" : "reset-unknown"}">
        ${knownReset
          ? `<div class="reset-kicker">${t("resetsIn")}</div>
             <div class="reset-val" data-reset-relative data-reset-at="${resetAt}">${escapeHtml(formatResetRelative(quota.resetAt, Date.now(), lang))}</div>
             <div class="reset-abs" data-reset-absolute data-reset-at="${resetAt}">${escapeHtml(formatResetAbsolute(quota.resetAt, lang))}</div>`
          : `<div class="reset-state">${escapeHtml(resetState(quota))}</div>`}
      </div>
    </div>
    <div class="bar"><div class="fill" style="width:${percent ?? 0}%"></div></div>
    ${temporaryLimit(quota)}
    ${pace}
  </article>`;
}

function addedProviders(dash: Dashboard): ProviderSnapshot[] {
  return dash.providers.filter((provider) => dash.catalog.find((item) => item.id === provider.id)?.enabled);
}

function loadingHtml(id: string, name: string): string {
  const started = loadingStarted.get(id) ?? Date.now();
  loadingStarted.set(id, started);
  return `<section class="provider-loading" aria-live="polite" aria-label="${escapeHtml(t("loadingProvider").replace("{name}", name))}">
    <div class="loading-copy"><span class="loading-spinner" aria-hidden="true"></span><span data-loading-copy data-loading-start="${started}">${escapeHtml(t("loadingProvider").replace("{name}", name))}</span></div>
    <span class="skeleton skeleton-title"></span>
    <span class="skeleton skeleton-value"></span>
    <span class="skeleton skeleton-bar"></span>
    <span class="skeleton skeleton-copy"></span>
  </section>`;
}

function updatingBadge(id: string): string {
  const started = loadingStarted.get(id) ?? Date.now();
  loadingStarted.set(id, started);
  return `<div class="provider-updating" role="status"><span class="loading-spinner" aria-hidden="true"></span><span data-loading-copy data-loading-start="${started}">${t("updating")}</span></div>`;
}

function relativeAge(timestamp: string | null | undefined): string {
  const elapsed = timestamp ? elapsedLabel(timestamp) : null;
  return elapsed?.unit === "minutes"
    ? t("minutesShort").replace("{count}", String(elapsed.count))
    : t("justNow");
}

function staleHint(provider: ProviderSnapshot): string {
  return `<p class="hint">${escapeHtml(t("staleDataFrom").replace("{time}", relativeAge(provider.updatedAt)))}</p>`;
}

function providerLogo(id: string, name: string): string {
  const visual = providerVisual(id, name);
  return `<span class="provider-logo-wrap"><img class="provider-logo" src="${visual.icon}" alt="" />${visual.badge ? `<span class="provider-badge">${visual.badge}</span>` : ""}</span>`;
}

function errorCard(provider: ProviderSnapshot, vendor: VendorInfo): string {
  const normalized = normalizeProviderError(provider, vendor);
  if (!normalized) return "";
  const action = normalized.action === "retry"
    ? "refresh"
    : normalized.action === "login"
      ? "provider-login"
      : normalized.action === "configure_credentials"
        ? "configure-provider"
        : "settings";
  const details = normalized.technicalDetails
    ? `<details class="technical-details"><summary>${t("showDetails")}</summary><pre>${escapeHtml(normalized.technicalDetails)}</pre></details>`
    : "";
  return `<section class="provider-error severity-${normalized.severity}">
    <h3>${escapeHtml(normalized.title)}</h3>
    <p>${escapeHtml(normalized.message)}</p>
    ${normalized.actionLabel ? `<button class="btn error-action" data-act="${action}" data-provider-id="${escapeHtml(vendor.id)}">${escapeHtml(normalized.actionLabel)}</button>` : ""}
    ${details}
  </section>`;
}

type DashboardSection = "details" | "products" | "actions";

function sectionStorageKey(providerId: string, section: DashboardSection): string {
  return `dashboard-section:${providerId}:${section}`;
}

function sectionIsOpen(providerId: string, section: DashboardSection, defaultOpen: boolean): boolean {
  try {
    const value = localStorage.getItem(sectionStorageKey(providerId, section));
    return value === null ? defaultOpen : value === "1";
  } catch {
    return defaultOpen;
  }
}

function collapsibleSection(
  providerId: string,
  section: DashboardSection,
  title: string,
  body: string,
  defaultOpen = false,
): string {
  return `<details class="details collapsible-section" data-dashboard-section="${section}" data-provider-id="${escapeHtml(providerId)}" ${sectionIsOpen(providerId, section, defaultOpen) ? "open" : ""}>
    <summary class="section-summary"><span>${escapeHtml(title)}</span><span class="section-chevron">${actionIconSvg("chevron-right", 14)}</span></summary>
    <div class="section-content">${body}</div>
  </details>`;
}

export function actionsSectionHtml(provider: ProviderSnapshot, vendor: VendorInfo): string {
  const actions = getProviderActions(vendor);
  const rows = actions.map((a) => {
    const iconHtml = actionIconSvg(a.icon, 14);
    const indicatorHtml = actionIconSvg(a.indicator, 14);
    const label = t(a.labelKey);
    if (a.kind === "external" && a.url) {
      return `<button class="action-row" data-open-url="${escapeHtml(a.url)}">
        <span class="action-icon">${iconHtml}</span>
        <span class="action-label">${escapeHtml(label)}</span>
        <span class="action-ext">${indicatorHtml}</span>
      </button>`;
    }
    if (a.kind === "configure") {
      return `<button class="action-row" data-act="configure-provider" data-provider-id="${escapeHtml(vendor.id)}">
        <span class="action-icon">${iconHtml}</span>
        <span class="action-label">${escapeHtml(label)}</span>
        <span class="action-ext">${indicatorHtml}</span>
      </button>`;
    }
    if (a.kind === "diagnosis") {
      return `<button class="action-row" data-act="copy-provider-diagnosis" data-provider-id="${escapeHtml(vendor.id)}">
        <span class="action-icon">${iconHtml}</span>
        <span class="action-label">${escapeHtml(label)}</span>
        <span class="action-ext">${indicatorHtml}</span>
      </button>`;
    }
    return "";
  });

  const cliCmd = getProviderCliCommand(vendor);

  return collapsibleSection(provider.id, "actions", t("actions"), `
    <div class="actions-list">${rows.join("")}</div>
    <div class="cli-card">
      <div class="cli-header">
        <span class="cli-icon">${actionIconSvg("terminal", 14)}</span>
        <span class="cli-title">${t("copyCliCmd")}</span>
      </div>
      <div class="cli-box" data-act="copy-cli-command" data-cli-cmd="${escapeHtml(cliCmd)}" role="button" tabindex="0" title="${t("copyCliCmd")}">
        <code class="cli-code">${escapeHtml(cliCmd)}</code>
        <button class="cli-copy-btn" data-act="copy-cli-command" data-cli-cmd="${escapeHtml(cliCmd)}" title="${t("copyCliCmd")}" aria-label="${t("copyCliCmd")}">
          ${actionIconSvg("copy", 14)}
        </button>
      </div>
    </div>
  `);
}

function connectionLabel(provider: ProviderSnapshot, vendor?: VendorInfo): string {
  if (vendor) return formatStatusText(deriveProviderState(vendor, provider, false));
  switch (provider.status) {
    case "connected": return t("statusConnected");
    case "needs_permission": return t("statusNeedPermission");
    case "unavailable": return t("statusUnavailable");
    case "error": return t("statusError");
    case "needs_auth":
    default:
      // vendor is always undefined here (the truthy case returned above).
      if (provider.statusReason === "invalid_credential") return t("statusSessionInvalid");
      return t("statusNeedLogin");
  }
}

function sourceLabel(source: string | null | undefined): string {
  switch ((source || "").toLowerCase()) {
    case "oauth": return t("viaOAuth");
    case "api": return t("viaApiKey");
    case "cli": return t("viaCli");
    case "local-session":
    case "local": return t("viaLocal");
    case "web-session":
    case "web": return t("viaWeb");
    case "auto": return t("autoSource");
    default: return t("viaMixed");
  }
}

function connectionHtml(provider: ProviderSnapshot, vendor?: VendorInfo): string {
  const status = escapeHtml(connectionLabel(provider, vendor));
  return provider.activeSource
    ? `${status} · ${escapeHtml(sourceLabel(provider.activeSource))}`
    : status;
}

function detailHtml(provider: ProviderSnapshot, vendor: VendorInfo, updating: boolean): string {
  const main = provider.quotas.slice(0, 2);
  let body = "";
  if (provider.status !== "connected") {
    body = errorCard(provider, vendor);
    if (provider.stale && main.length) {
      body += main.map(progressBlock).join("");
      body += staleHint(provider);
    }
  } else {
    body = `<div class="quota-grid">${main.map(progressBlock).join("")}</div>`;
    if (provider.stale) body += staleHint(provider);
  }

  const additionalQuotas = provider.quotas.slice(2).map((quota) => {
    const value = pctOf(quota);
    return `<div class="kv"><span>${escapeHtml(quota.label)}</span><span>${value == null ? "—" : `${Math.round(value)}% ${t("used")}`}</span></div>`;
  }).join("");
  const costEstimated = provider.cost?.confidence === "estimated";
  const legacyRows = provider.lines
    .filter((line): line is Exclude<MetricLine, { kind: "progress" }> =>
      line.kind !== "progress" && !["credits", "resets"].includes(line.id))
    .map((line) => {
      const suffix = costEstimated && line.id.startsWith("cost") ? ` · ${t("estimated")}` : "";
      return `<div class="kv"><span>${escapeHtml(line.label)}</span><span>${escapeHtml(line.text)}${escapeHtml(suffix)}</span></div>`;
    })
    .join("");
  const creditRows = provider.credits
    ? `<div class="kv"><span>${t("credits")}</span><span>${provider.credits.remaining.toLocaleString(lang === "es" ? "es-ES" : "en-US", { maximumFractionDigits: 0 })}</span></div>
       ${provider.credits.resetsAvailable == null ? "" : `<div class="kv"><span>${t("resetsAvailable")}</span><span>${provider.credits.resetsAvailable}</span></div>`}`
    : "";
  const redeemReset = provider.id === "openai" && (provider.credits?.resetsAvailable ?? 0) > 0 && vendor.links.usageUrl
    ? `<button class="btn ghost" data-redeem-reset="${escapeHtml(provider.id)}" data-reset-url="${escapeHtml(vendor.links.usageUrl)}">${escapeHtml(t("redeemReset"))}</button>`
    : "";
  const breakdown = provider.productBreakdown.length
    ? collapsibleSection(provider.id, "products", t("productUsage"), `
        ${provider.productBreakdown.map((item) => `<div class="kv"><span>${escapeHtml(item.name)}</span><span>${Math.round(item.usedPercent)}%</span></div>`).join("")}
       `)
    : "";
  const details = collapsibleSection(provider.id, "details", t("details"), `
      <div class="kv"><span>${t("connection")}</span><span>${connectionHtml(provider, vendor)}</span></div>
      ${creditRows}
      ${additionalQuotas}
      ${legacyRows}
      ${redeemReset}
    `, true);

  return `<div class="detail-inner">
    ${updating ? updatingBadge(provider.id) : ""}
    ${body}
    ${details}
    ${breakdown}
    ${actionsSectionHtml(provider, vendor)}
  </div>`;
}

function compactDetailHtml(provider: ProviderSnapshot, vendor: VendorInfo, dash: Dashboard): string {
  const quotas = provider.quotas.slice(0, 2);
  const rows = quotas.map((quota) => {
    const used = pctOf(quota);
    const reset = quota.resetAt ? formatResetRelative(quota.resetAt, Date.now(), lang) : resetState(quota);
    return `<div class="compact-quota">
      <span class="compact-quota-label">${escapeHtml(quota.label)}</span>
      <strong>${used == null ? "—" : `${Math.round(used)}%`}</strong>
      <span data-reset-relative data-reset-at="${escapeHtml(quota.resetAt || "")}">${escapeHtml(reset)}</span>
    </div>`;
  }).join("");
  const rec = (dash.recommendAction && dash.recommendAction !== "insufficient_data"
    ? recCopy(dash, provider.id)
    : null) ?? (dash.recommendName && dash.recommendLeft != null
    ? {
      text: `${dash.recommendName} · ${Math.round(dash.recommendLeft)}% ${t("available")}`,
      selectId: dash.recommendId || "",
    }
    : null);
  const recommendation = rec
    ? `<button class="compact-recommendation" data-select="${escapeHtml(rec.selectId)}" title="${escapeHtml(recWhyTitle(dash))}">${escapeHtml(recPrefix(dash))}${escapeHtml(rec.text)}</button>`
    : "";

  const quickLinks = getCompactQuickActions(vendor).map((act) => {
    if (act.isConfigure) {
      return `<button class="compact-action-btn" data-act="configure-provider" data-provider-id="${escapeHtml(vendor.id)}">${t(act.labelKey)}</button>`;
    }
    return `<button class="compact-action-btn" data-open-url="${escapeHtml(act.url || "")}">${t(act.labelKey)}</button>`;
  });

  return `<div class="compact-card">
    <div class="compact-provider">${providerLogo(vendor.id, vendor.name)}<strong>${escapeHtml(vendor.name)}</strong>${provider.stale ? `<span>${t("updating")}</span>` : ""}</div>
    ${rows || `<p class="hint">${t("notAvailable")}</p>`}
    ${recommendation}
    <div class="compact-actions">${quickLinks.join("")}</div>
  </div>`;
}

export function updateResetClocks(root: ParentNode = document): void {
  root.querySelectorAll<HTMLElement>("[data-reset-relative]").forEach((element) => {
    element.textContent = formatResetRelative(element.dataset.resetAt, Date.now(), lang);
  });
  root.querySelectorAll<HTMLElement>("[data-reset-absolute]").forEach((element) => {
    element.textContent = formatResetAbsolute(element.dataset.resetAt, lang);
  });
}

function updateTabEdges(): void {
  const tabs = $("tabs");
  const viewport = $("tabs-scroll");
  viewport.classList.toggle("can-scroll-left", tabs.scrollLeft > 1);
  viewport.classList.toggle(
    "can-scroll-right",
    tabs.scrollLeft + tabs.clientWidth < tabs.scrollWidth - 1,
  );
}

function wireProviderScroll(): void {
  const tabs = $("tabs");
  tabs.onscroll = () => {
    updateTabEdges();
    if (tabs.scrollLeft > 4) {
      localStorage.setItem("providerScrollSeen", "1");
      $("tabs-hint").classList.add("hidden");
    }
  };
  tabs.onwheel = (event) => {
    if (tabs.scrollWidth <= tabs.clientWidth) return;
    event.preventDefault();
    tabs.scrollLeft += horizontalWheelDelta(event.deltaX, event.deltaY);
  };
}

export function updateLoadingClocks(root: ParentNode = document): void {
  root.querySelectorAll<HTMLElement>("[data-loading-copy]").forEach((element) => {
    const started = Number(element.dataset.loadingStart || Date.now());
    if (Date.now() - started >= 6_000) element.textContent = t("takingLonger");
  });
}

/// Renders the dashboard view. Returns the effective selected provider id
/// (may differ from the input when the previous selection is gone).
export function renderDash(dash: Dashboard | null, selectedId: string): string {
  if (!dash) return selectedId;
  const added = addedProviders(dash);
  // "Primario" (Ajustes → General) doubles as tab order: it leads the rail
  // instead of only picking the initial selection.
  const enabled = dash.catalog
    .filter((vendor) => vendor.enabled)
    .sort((a, b) => (a.id === dash.primary ? -1 : b.id === dash.primary ? 1 : 0));
  $("empty").classList.toggle("hidden", enabled.length > 0);
  $("empty-text").textContent = t("empty");
  $("empty-detect").textContent = t("detect");
  $("empty-add").textContent = t("addManual");
  $("btn-quit").textContent = t("quit");
  document.querySelectorAll<HTMLButtonElement>('[data-act="refresh"]').forEach((button) => {
    button.disabled = dash.refreshing;
    button.classList.toggle("is-refreshing", dash.refreshing);
    button.toggleAttribute("aria-busy", dash.refreshing);
  });
  if (!enabled.find((vendor) => vendor.id === selectedId) && enabled.length) {
    selectedId = dash.primary && enabled.some((vendor) => vendor.id === dash.primary) ? dash.primary : enabled[0].id;
    localStorage.setItem("selected", selectedId);
  }

  const selected = enabled.find((vendor) => vendor.id === selectedId) ?? enabled[0];
  document.documentElement.style.setProperty(
    "--accent",
    providerVisual(selected?.id || "unknown", selected?.name || "AI").accent,
  );

  const tabs = $("tabs");
  const previousTabScroll = tabs.scrollLeft;
  const shouldRevealSelection = selectedId !== lastRevealedProvider;
  tabs.innerHTML = enabled.map((vendor) => {
    const active = vendor.id === selectedId;
    const snapshot = added.find((provider) => provider.id === vendor.id);
    const isLoading = dash.loadingProviders.includes(vendor.id);
    const visual = providerVisual(vendor.id, vendor.name);
    return tabHtml({
      vendor,
      snapshot,
      active,
      loading: isLoading,
      visual: { icon: visual.icon, accent: visual.accent },
      logoHtml: providerLogo(vendor.id, vendor.name),
    });
  }).join("");
  $("add-provider").setAttribute("title", t("add"));
  $("add-provider").setAttribute("aria-label", t("add"));
  wireProviderScroll();
  requestAnimationFrame(() => {
    const active = shouldRevealSelection ? tabs.querySelector<HTMLElement>(".tab.active") : null;
    if (active) {
      const delta = activeItemScrollDelta(tabs.getBoundingClientRect(), active.getBoundingClientRect());
      if (delta) tabs.scrollBy({ left: delta, behavior: "smooth" });
    } else tabs.scrollLeft = previousTabScroll;
    lastRevealedProvider = selectedId;
    updateTabEdges();
    const overflowing = tabs.scrollWidth > tabs.clientWidth + 1;
    const showHint = overflowing && localStorage.getItem("providerScrollSeen") !== "1";
    $("tabs-hint").textContent = t("scrollMore");
    $("tabs-hint").classList.toggle("hidden", !showHint);
  });

  const current = added.find((provider) => provider.id === selectedId);
  const currentVendor = enabled.find((vendor) => vendor.id === selectedId);
  const currentLoading = !!currentVendor && dash.loadingProviders.includes(currentVendor.id);
  if (!currentLoading && currentVendor) loadingStarted.delete(currentVendor.id);
  $("detail").innerHTML = current && currentVendor
    ? dash.compactMode ? compactDetailHtml(current, currentVendor, dash) : detailHtml(current, currentVendor, currentLoading)
    : currentVendor ? loadingHtml(currentVendor.id, currentVendor.name) : "";
  $("detail").classList.toggle("hidden", !currentVendor);

  const stamp = current?.updatedAt ? new Date(current.updatedAt) : null;
  if (currentLoading) {
    $("updated").textContent = current ? t("updating") : t("updatingProvider").replace("{name}", currentVendor?.short || currentVendor?.name || "");
  } else if (stamp && !Number.isNaN(stamp.getTime())) {
    const age = relativeAge(current!.updatedAt);
    $("updated").textContent = current?.stale
      ? current.error && current.lastAttemptAt
        ? t("refreshFailedAttempt").replace("{time}", relativeAge(current.lastAttemptAt))
        : t("lastUpdatedAgo").replace("{time}", age)
      : `${t("updated")} ${stamp.toLocaleTimeString(lang === "es" ? "es-ES" : "en-US", { hour: "numeric", minute: "2-digit" })}`;
  }
  if (!stamp || Number.isNaN(stamp.getTime())) $("updated").textContent = "";

  document.querySelectorAll<HTMLElement>(".seg").forEach((element) => {
    element.classList.toggle("active", (element.dataset.act === "lang-en" && lang === "en") || (element.dataset.act === "lang-es" && lang === "es"));
  });

  const stall = $("stall");
  const stallRec = added.length >= 2 ? recCopy(dash, selectedId) : null;
  if (stallRec) {
    stall.textContent = `${recPrefix(dash)}${stallRec.text}`;
    stall.classList.remove("hidden");
    stall.dataset.select = stallRec.selectId;
    stall.title = recWhyTitle(dash);
  } else {
    stall.textContent = "";
    stall.classList.add("hidden");
    delete stall.dataset.select;
    stall.removeAttribute("title");
  }

  return selectedId;
}
