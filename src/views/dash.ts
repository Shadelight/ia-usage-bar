// Main dashboard view: provider tabs + the selected provider's detail.

import { $, escapeHtml } from "../api";
import type { Dashboard, MetricLine, ProviderSnapshot, UsageQuota, VendorInfo } from "../api";
import { normalizeProviderError } from "../errors";
import { lang, t } from "../i18n";
import { providerVisual } from "../providers";
import { formatResetAbsolute, formatResetRelative } from "../reset-format";

function pctOf(quota: UsageQuota): number | null {
  if (quota.usedPercent != null) return Math.max(0, Math.min(100, quota.usedPercent));
  if (quota.usedAmount != null && quota.limitAmount != null && quota.limitAmount > 0) {
    return Math.max(0, Math.min(100, (quota.usedAmount / quota.limitAmount) * 100));
  }
  return null;
}

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

function paceNote(quota: UsageQuota): string {
  if (!quota.resetAt) return "";
  const duration = windowSeconds(quota);
  if (duration <= 0) return "";
  const end = new Date(quota.resetAt).getTime();
  if (!Number.isFinite(end)) return "";
  const fraction = Math.max(0, Math.min(1, (duration * 1_000 - (end - Date.now())) / (duration * 1_000)));
  const utilization = pctOf(quota);
  if (fraction < 0.05 || utilization == null || utilization < 8 || utilization / fraction < 100) return "";
  const remainingMs = ((100 - utilization) / Math.max(utilization / (fraction * duration), 1e-9)) * 1_000;
  const hours = Math.floor(remainingMs / 3_600_000);
  const minutes = Math.round((remainingMs % 3_600_000) / 60_000);
  return `${t("pace")} ${hours}h ${minutes}m`;
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
  const pace = paceNote(quota);
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
    ${pace ? `<div class="pace">⚠ ${escapeHtml(pace)}</div>` : ""}
  </article>`;
}

function addedProviders(dash: Dashboard): ProviderSnapshot[] {
  return dash.providers.filter((provider) => dash.catalog.find((item) => item.id === provider.id)?.enabled);
}

function loadingHtml(name: string): string {
  return `<section class="provider-loading" aria-live="polite" aria-label="${escapeHtml(t("loadingProvider").replace("{name}", name))}">
    <span class="skeleton skeleton-title"></span>
    <span class="skeleton skeleton-value"></span>
    <span class="skeleton skeleton-bar"></span>
    <span class="skeleton skeleton-copy"></span>
  </section>`;
}

function providerLogo(id: string, name: string): string {
  const visual = providerVisual(id, name);
  return `<span class="provider-logo-wrap"><img class="provider-logo" src="${visual.icon}" alt="" />${visual.badge ? `<span class="provider-badge">${visual.badge}</span>` : ""}</span>`;
}

function errorCard(provider: ProviderSnapshot, vendor: VendorInfo): string {
  const normalized = normalizeProviderError(provider, vendor);
  if (!normalized) return "";
  const action = normalized.action === "retry" ? "refresh" : "settings";
  const details = normalized.technicalDetails
    ? `<details class="technical-details"><summary>${t("showDetails")}</summary><pre>${escapeHtml(normalized.technicalDetails)}</pre></details>`
    : "";
  return `<section class="provider-error severity-${normalized.severity}">
    <h3>${escapeHtml(normalized.title)}</h3>
    <p>${escapeHtml(normalized.message)}</p>
    ${normalized.actionLabel ? `<button class="btn error-action" data-act="${action}">${escapeHtml(normalized.actionLabel)}</button>` : ""}
    ${details}
  </section>`;
}

function detailHtml(provider: ProviderSnapshot, vendor: VendorInfo): string {
  const main = provider.quotas.slice(0, 2);
  let body = "";
  if (provider.status !== "connected") {
    body = errorCard(provider, vendor);
    if (provider.stale && main.length) {
      body += main.map(progressBlock).join("");
      body += `<p class="hint">${t("stale")}</p>`;
    }
  } else {
    body = main.map(progressBlock).join("");
    if (provider.stale) body += `<p class="hint">${t("stale")}</p>`;
  }

  const additionalQuotas = provider.quotas.slice(2).map((quota) => {
    const value = pctOf(quota);
    return `<div class="kv"><span>${escapeHtml(quota.label)}</span><span>${value == null ? "—" : `${Math.round(value)}% ${t("used")}`}</span></div>`;
  }).join("");
  const legacyRows = provider.lines
    .filter((line): line is Exclude<MetricLine, { kind: "progress" }> =>
      line.kind !== "progress" && !["credits", "resets"].includes(line.id))
    .map((line) => `<div class="kv"><span>${escapeHtml(line.label)}</span><span>${escapeHtml(line.text)}</span></div>`)
    .join("");
  const creditRows = provider.credits
    ? `<div class="kv"><span>${t("credits")}</span><span>${provider.credits.remaining.toLocaleString(lang === "es" ? "es-ES" : "en-US", { maximumFractionDigits: 0 })}</span></div>
       ${provider.credits.resetsAvailable == null ? "" : `<div class="kv"><span>${t("resetsAvailable")}</span><span>${provider.credits.resetsAvailable}</span></div>`}`
    : "";
  const breakdown = provider.productBreakdown.length
    ? `<section class="details breakdown">
        <h3>${t("productUsage")}</h3>
        ${provider.productBreakdown.map((item) => `<div class="kv"><span>${escapeHtml(item.name)}</span><span>${Math.round(item.usedPercent)}%</span></div>`).join("")}
       </section>`
    : "";

  return `${body}
    <section class="details">
      <h3>${t("details")}</h3>
      <div class="kv"><span>${t("plan")}</span><span>${escapeHtml(provider.plan || "—")}</span></div>
      ${creditRows}
      ${additionalQuotas}
      ${legacyRows}
    </section>
    ${breakdown}`;
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

/// Renders the dashboard view. Returns the effective selected provider id
/// (may differ from the input when the previous selection is gone).
export function renderDash(dash: Dashboard | null, selectedId: string): string {
  if (!dash) return selectedId;
  const added = addedProviders(dash);
  const enabled = dash.catalog.filter((vendor) => vendor.enabled);
  $("empty").classList.toggle("hidden", enabled.length > 0);
  $("empty-text").textContent = t("empty");
  $("empty-detect").textContent = t("detect");
  $("empty-add").textContent = t("addManual");
  $("btn-quit").textContent = t("quit");
  if (!enabled.find((vendor) => vendor.id === selectedId) && enabled.length) {
    selectedId = dash.primary && enabled.some((vendor) => vendor.id === dash.primary) ? dash.primary : enabled[0].id;
    localStorage.setItem("selected", selectedId);
  }

  const selected = enabled.find((vendor) => vendor.id === selectedId) ?? enabled[0];
  document.documentElement.style.setProperty(
    "--accent",
    providerVisual(selected?.id || "unknown", selected?.name || "AI").accent,
  );

  $("tabs").innerHTML = enabled.map((vendor) => {
    const active = vendor.id === selectedId;
    const snapshot = added.find((provider) => provider.id === vendor.id);
    const status = snapshot?.status || "loading";
    const visual = providerVisual(vendor.id, vendor.name);
    return `<button class="tab ${active ? "active" : ""}" data-select="${escapeHtml(vendor.id)}" style="--provider-accent:${visual.accent}">
      ${providerLogo(vendor.id, vendor.name)}
      <span class="label">${escapeHtml(vendor.name)}</span>
      <span class="provider-status-dot status-${status}" title="${status}"></span>
    </button>`;
  }).join("");
  $("add-provider").setAttribute("title", t("add"));
  $("add-provider").setAttribute("aria-label", t("add"));
  const tabs = $("tabs");
  tabs.onscroll = updateTabEdges;
  requestAnimationFrame(() => {
    tabs.querySelector<HTMLElement>(".tab.active")?.scrollIntoView({ block: "nearest", inline: "nearest" });
    updateTabEdges();
  });

  const current = added.find((provider) => provider.id === selectedId);
  const currentVendor = enabled.find((vendor) => vendor.id === selectedId);
  $("detail").innerHTML = current && currentVendor
    ? detailHtml(current, currentVendor)
    : currentVendor ? loadingHtml(currentVendor.name) : "";
  $("detail").classList.toggle("hidden", !currentVendor);

  const stamp = current?.updatedAt ? new Date(current.updatedAt) : null;
  if (stamp && !Number.isNaN(stamp.getTime())) {
    $("updated").textContent = `${t("updated")} ${stamp.toLocaleTimeString(lang === "es" ? "es-ES" : "en-US", { hour: "numeric", minute: "2-digit" })}`;
  }
  if (!stamp || Number.isNaN(stamp.getTime())) $("updated").textContent = "";

  document.querySelectorAll<HTMLElement>(".seg").forEach((element) => {
    element.classList.toggle("active", (element.dataset.act === "lang-en" && lang === "en") || (element.dataset.act === "lang-es" && lang === "es"));
  });

  const stall = $("stall");
  if (added.length >= 2 && dash.recommendName && dash.recommendLeft != null) {
    const template = dash.recommendId === selectedId ? t("stallHere") : t("stall");
    stall.textContent = template.replace("{name}", dash.recommendName).replace("{left}", String(Math.round(dash.recommendLeft)));
    stall.classList.remove("hidden");
    stall.dataset.select = dash.recommendId || "";
  } else {
    stall.textContent = "";
    stall.classList.add("hidden");
    delete stall.dataset.select;
  }

  return selectedId;
}
