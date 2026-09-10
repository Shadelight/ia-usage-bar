// Main dashboard view: provider tabs + the selected provider's detail.

import { $, accent, escapeHtml, glyph, tabName } from "../api";
import type { Dashboard, MetricLine, ProviderSnapshot } from "../api";
import { lang, t } from "../i18n";

function pctOf(line: Extract<MetricLine, { kind: "progress" }>): number {
  if (line.format === "percent") return Math.max(0, Math.min(150, line.used));
  if (line.limit > 0) return (line.used / line.limit) * 100;
  return 0;
}

function windowLabel(secs: number): string {
  if (secs > 0 && secs <= 6 * 3600) {
    const h = Math.max(1, Math.round(secs / 3600));
    return lang === "es" ? `ventana ${h}h` : `${h}h ${t("window")}`;
  }
  if (secs > 0 && secs <= 36 * 3600) return t("daily");
  if (secs >= 20 * 86400) return t("monthly");
  return t("weekly");
}

function resetCountdown(line: Extract<MetricLine, { kind: "progress" }>): string {
  if (!line.resetsAt) return line.resetsInLabel || "—";
  const end = new Date(line.resetsAt).getTime();
  if (isNaN(end)) return line.resetsInLabel || "—";
  const secs = Math.max(0, Math.round((end - Date.now()) / 1000));
  const h = Math.floor(secs / 3600);
  const m = Math.floor((secs % 3600) / 60);
  if (h > 0) return `${h}h ${m}m`;
  return `${m}m`;
}

function exactReset(iso?: string | null): string {
  if (!iso) return "—";
  const d = new Date(iso);
  if (isNaN(d.getTime())) return "—";
  return d.toLocaleString(lang === "es" ? "es-ES" : "en-US", {
    month: "numeric",
    day: "numeric",
    year: "2-digit",
    hour: "numeric",
    minute: "2-digit",
  });
}

function paceNote(line: Extract<MetricLine, { kind: "progress" }>): string {
  if (!line.resetsAt || line.windowSecs <= 0) return "";
  const end = new Date(line.resetsAt).getTime();
  if (isNaN(end)) return "";
  const frac = Math.max(
    0,
    Math.min(1, (line.windowSecs * 1000 - (end - Date.now())) / (line.windowSecs * 1000)),
  );
  if (frac < 0.05) return "";
  const util = pctOf(line);
  if (util < 8) return "";
  const projected = util / frac;
  if (projected < 100) return "";
  const remainMs = ((100 - util) / Math.max(util / (frac * line.windowSecs), 1e-9)) * 1000;
  const h = Math.floor(remainMs / 3600000);
  const m = Math.round((remainMs % 3600000) / 60000);
  return `${t("pace")} ${h}h ${m}m`;
}

function counts(line: Extract<MetricLine, { kind: "progress" }>): string {
  if (line.format === "percent") {
    const used = Math.round(line.used);
    return `${used} / 100`;
  }
  return `${Math.round(line.used)} / ${Math.round(line.limit)}`;
}

function addedProviders(dash: Dashboard): ProviderSnapshot[] {
  return dash.providers.filter((p) => dash.catalog.find((c) => c.id === p.id)?.enabled);
}

function progressBlock(line: Extract<MetricLine, { kind: "progress" }>): string {
  const pct = Math.min(100, pctOf(line));
  const left = Math.max(0, Math.round(100 - pct));
  const pace = paceNote(line);
  return `<article class="block">
    <div class="block-top">
      <div>
        <div class="kicker">${escapeHtml(line.label)}</div>
        <div class="pct-row">
          <span class="pct">${Math.round(pct)}%</span>
          <span class="remain">${left}% ${t("remaining")}</span>
        </div>
      </div>
      <div class="reset-col">
        <div class="reset-kicker">${t("resetsIn")}</div>
        <div class="reset-val">${escapeHtml(resetCountdown(line))}</div>
      </div>
    </div>
    <div class="bar"><div class="fill" style="width:${pct}%"></div></div>
    <div class="meta">
      <span>${escapeHtml(counts(line))}</span>
      <span>${escapeHtml(windowLabel(line.windowSecs))}</span>
    </div>
    ${pace ? `<div class="pace">⚠ ${escapeHtml(pace)}</div>` : ""}
  </article>`;
}

function detailHtml(p: ProviderSnapshot): string {
  const progress = p.lines.filter((l): l is Extract<MetricLine, { kind: "progress" }> => l.kind === "progress");
  const preferred = progress.filter((l) => l.visible !== "demand");
  const main = (preferred.length ? preferred : progress).slice(0, 2);
  const mainIds = new Set(main.map((l) => l.id));
  const weekly = [...progress].reverse().find((l) => l.windowSecs >= 6 * 86400) || progress[progress.length - 1];
  const cap = p.plan || p.lines.find((l) => l.kind !== "progress")?.text || "—";
  const resetIso = weekly?.resetsAt;
  let body = "";
  if (!p.connected) {
    body = `${p.error ? `<p class="err-msg">${escapeHtml(p.error)}</p>` : ""}
      ${p.hint ? `<p class="hint">${escapeHtml(p.hint)}</p>` : ""}`;
  } else {
    body = main.map(progressBlock).join("");
    if (p.stale) body += `<p class="hint">${t("stale")}</p>`;
  }
  const extraRows = p.lines
    .filter((l) => l.kind !== "progress" || !mainIds.has(l.id))
    .map((l) => {
      const value = l.kind === "progress" ? `${Math.round(Math.min(100, pctOf(l)))}%` : l.text;
      return `<div class="kv"><span>${escapeHtml(l.label)}</span><span>${escapeHtml(value)}</span></div>`;
    })
    .join("");
  return `${body}
    <section class="details">
      <h3>${t("details")}</h3>
      <div class="kv"><span>${t("cap")}</span><span>${escapeHtml(String(cap))}</span></div>
      <div class="kv"><span>${t("resets")}</span><span>${escapeHtml(exactReset(resetIso))}</span></div>
      ${extraRows}
    </section>`;
}

/// Renders the dashboard view. Returns the effective selected provider id
/// (may differ from the input when the previous selection is gone).
export function renderDash(dash: Dashboard | null, selectedId: string): string {
  if (!dash) return selectedId;
  const added = addedProviders(dash);
  $("empty").classList.toggle("hidden", added.length > 0);
  $("empty-text").textContent = t("empty");
  $("empty-detect").textContent = t("detect");
  $("empty-add").textContent = t("addManual");
  $("btn-quit").textContent = t("quit");
  document.documentElement.style.setProperty("--accent", accent(selectedId || added[0]?.id || "anthropic"));

  if (!added.find((p) => p.id === selectedId) && added.length) {
    selectedId = dash.primary && added.some((p) => p.id === dash.primary) ? dash.primary : added[0].id;
    localStorage.setItem("selected", selectedId);
  }

  $("tabs").innerHTML =
    added
      .map((p) => {
        const on = p.id === selectedId;
        return `<button class="tab ${on ? "active" : ""}" data-select="${p.id}" style="--accent:${accent(p.id)}">
          <span class="glyph">${glyph(p.id)}</span>
          <span class="label">${escapeHtml(tabName(p.id, p.name))}</span>
        </button>`;
      })
      .join("") +
    `<button class="tab add" data-act="settings" title="${t("add")}">
      <span class="glyph"><svg viewBox="0 0 16 16"><path d="M8 3v10M3 8h10" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round"/></svg></span>
    </button>`;

  const current = added.find((p) => p.id === selectedId);
  $("detail").innerHTML = current ? detailHtml(current) : "";
  $("detail").classList.toggle("hidden", !current);

  const stamp = current?.updatedAt ? new Date(current.updatedAt) : null;
  if (stamp && !isNaN(stamp.getTime())) {
    $("updated").textContent = `${t("updated")} ${stamp.toLocaleTimeString(lang === "es" ? "es-ES" : "en-US", {
      hour: "numeric",
      minute: "2-digit",
    })}`;
  }

  document.querySelectorAll<HTMLElement>(".seg").forEach((el) => {
    el.classList.toggle(
      "active",
      (el.dataset.act === "lang-en" && lang === "en") || (el.dataset.act === "lang-es" && lang === "es"),
    );
  });

  const stall = $("stall");
  const recName = dash.recommendName;
  const recLeft = dash.recommendLeft;
  if (added.length >= 2 && recName && recLeft != null) {
    const tpl = dash.recommendId === selectedId ? t("stallHere") : t("stall");
    stall.textContent = tpl.replace("{name}", recName).replace("{left}", String(Math.round(recLeft)));
    stall.classList.remove("hidden");
    stall.dataset.select = dash.recommendId || "";
  } else {
    stall.textContent = "";
    stall.classList.add("hidden");
    delete stall.dataset.select;
  }

  return selectedId;
}
