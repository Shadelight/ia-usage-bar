import "./styles.css";

type MetricLine =
  | {
      kind: "progress";
      id: string;
      label: string;
      used: number;
      limit: number;
      format: string;
      resetsAt?: string | null;
      resetsInLabel: string;
      windowSecs: number;
      visible: string;
    }
  | { kind: "values"; id: string; label: string; text: string; visible: string }
  | { kind: "badge"; id: string; label: string; text: string; visible: string };

interface ProviderSnapshot {
  id: string;
  name: string;
  short: string;
  plan: string;
  connected: boolean;
  stale: boolean;
  error: string | null;
  hint: string | null;
  updatedAt: string;
  lines: MetricLine[];
  primaryUtilization: number | null;
}

interface VendorInfo {
  id: string;
  name: string;
  short: string;
  authKind: string;
  envKey: string | null;
  hint: string;
  needsKey: boolean;
  enabled: boolean;
  detected: boolean;
}

interface SpendRow {
  id: string;
  name: string;
  label: string;
  usd: number;
}

interface Dashboard {
  providers: ProviderSnapshot[];
  catalog: VendorInfo[];
  refreshMinutes: number;
  primary: string;
  notifications: boolean;
  showUsageAs: string;
  resetTimes: string;
  nextUpdateInSecs: number;
  spendMonthUsd: number;
  spend: SpendRow[];
  recommendId: string | null;
  recommendName: string | null;
  recommendLeft: number | null;
}

const I18N: Record<string, Record<string, string>> = {
  es: {
    empty: "Aún no has añadido ninguna IA. Detecta las que ya tienes iniciadas o actívalas abajo.",
    detect: "Detectar las que ya uso",
    addManual: "Añadir manualmente",
    settings: "Ajustes",
    refresh: "Actualizar",
    providers: "IAs en la barra",
    providersHint: "Solo las que actives aparecen arriba. Nunca se apagan solas.",
    general: "General",
    notifications: "Notificaciones",
    interval: "Refresco",
    primary: "Primario",
    apiKey: "API key",
    save: "Guardar",
    stale: "Datos antiguos",
    remaining: "restante",
    resetsIn: "Reinicia en",
    details: "Detalles",
    cap: "Tope",
    resets: "Reinicio",
    quit: "Salir",
    updated: "Actualizado",
    session: "Sesión",
    weekly: "Semanal",
    monthly: "Mensual",
    daily: "Diario",
    window: "ventana",
    pace: "Al ritmo actual, el límite llega en",
    add: "Añadir",
    spend: "Gasto del mes",
    spendEmpty: "Aún no hay gasto en dólares de las IAs activas. Claude Code y las APIs con factura aparecen aquí.",
    stall: "Cambia a {name} — {left}% de margen",
    stallHere: "{name} tiene el mayor margen ({left}%)",
    guide: "Guía de conexión",
    guideHint: "Detecta logins locales o pega la key. No hace falta buscar archivos de config.",
    notifHint: "Avisos al 75%, 90% y 95%, y cuando se reinicia la ventana.",
  },
  en: {
    empty: "You haven’t added any AIs yet. Detect local logins or turn them on below.",
    detect: "Detect the ones I already use",
    addManual: "Add manually",
    settings: "Settings",
    refresh: "Refresh",
    providers: "AIs on the bar",
    providersHint: "Only the ones you enable show up above. We never turn one off for you.",
    general: "General",
    notifications: "Notifications",
    interval: "Refresh every",
    primary: "Primary",
    apiKey: "API key",
    save: "Save",
    stale: "Stale",
    remaining: "remaining",
    resetsIn: "Resets in",
    details: "Details",
    cap: "Cap",
    resets: "Resets",
    quit: "Quit",
    updated: "Updated",
    session: "Session",
    weekly: "Weekly",
    monthly: "Monthly",
    daily: "Daily",
    window: "window",
    pace: "At current pace, limit hit in",
    add: "Add",
    spend: "Monthly spend",
    spendEmpty: "No dollar spend yet from enabled AIs. Claude Code and billed APIs show up here.",
    stall: "Switch to {name} — {left}% headroom",
    stallHere: "{name} has the most headroom ({left}%)",
    guide: "Setup guide",
    guideHint: "Detect local logins or paste a key. No digging through config files.",
    notifHint: "Alerts at 75%, 90% and 95%, and when a window resets.",
  },
};

const ACCENT: Record<string, string> = {
  anthropic: "#ff7a3d",
  openai: "#34d399",
  openai_admin: "#2dd4bf",
  cursor: "#4ade80",
  copilot: "#a78bfa",
  antigravity: "#fb7185",
  grok: "#e5e7eb",
  supergrok: "#e5e7eb",
  openrouter: "#818cf8",
  zai: "#38bdf8",
  deepseek: "#60a5fa",
  kimi: "#f472b6",
  kilo: "#fbbf24",
  novita: "#c084fc",
  moonshot: "#94a3b8",
  minimax: "#f97316",
  kiro: "#22d3ee",
  nous: "#a3e635",
  opencode_go: "#facc15",
  commandcode: "#fb923c",
  anthropic_api: "#fdba74",
  groq: "#f59e0b",
  windsurf: "#38bdf8",
};

const TAB_NAME: Record<string, string> = {
  anthropic: "Claude",
  openai: "OpenAI",
  openai_admin: "API",
  cursor: "Cursor",
  copilot: "Copilot",
  antigravity: "Gemini",
  grok: "Grok",
  supergrok: "Grok",
  openrouter: "Router",
  zai: "Z.AI",
  deepseek: "DeepSeek",
  kimi: "Kimi",
  kilo: "Kilo",
  novita: "Novita",
  moonshot: "Kimi",
  minimax: "MiniMax",
  kiro: "Kiro",
  nous: "Nous",
  opencode_go: "Go",
  commandcode: "Cmd",
  anthropic_api: "Admin",
  groq: "Groq",
  windsurf: "Windsurf",
};

let lang = localStorage.getItem("lang") === "en" ? "en" : "es";
const t = (k: string) => I18N[lang][k] ?? k;
const $ = (id: string) => document.getElementById(id)!;

function isTauri(): boolean {
  return !!(window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
}

async function invokeCmd<T>(cmd: string, args?: Record<string, unknown>): Promise<T | null> {
  if (!isTauri()) return null;
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<T>(cmd, args);
}

let dash: Dashboard | null = null;
let selectedId = localStorage.getItem("selected") || "";
let view: "dash" | "spend" | "settings" = "dash";

function accent(id: string): string {
  return ACCENT[id] || "#60a5fa";
}

function tabName(id: string, fallback: string): string {
  return TAB_NAME[id] || fallback.split(" ")[0];
}

function glyph(id: string): string {
  const icons: Record<string, string> = {
    anthropic: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"><path d="M12 3v18M5.2 7.2l13.6 9.6M5.2 16.8l13.6-9.6"/></svg>`,
    openai: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7"><path d="M12 4.5c1.8-1 4-.7 5.4.9 1.2 1.3 1.5 3.2.9 4.8 1.5.9 2.3 2.7 1.9 4.4-.4 1.8-1.9 3.1-3.7 3.4-1 .2-2 .1-2.9-.3-1.8 1-4 .7-5.4-.9-1.2-1.3-1.5-3.2-.9-4.8-1.5-.9-2.3-2.7-1.9-4.4C6 6.8 7.5 5.5 9.3 5.2c.9-.2 1.8-.1 2.7.3z"/></svg>`,
    cursor: `<svg viewBox="0 0 24 24" fill="currentColor"><path d="M12 3 20 8v8l-8 5-8-5V8l8-5zm0 2.3L6.6 8.6v6.8L12 18.7l5.4-3.3V8.6L12 5.3z"/></svg>`,
    copilot: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7"><path d="M5 13v-1a7 7 0 0 1 14 0v1M5 14a3 3 0 0 0-3 3v1h6v-1a3 3 0 0 0-3-3zm14 0a3 3 0 0 1 3 3v1h-6v-1a3 3 0 0 1 3-3zM9 19h6"/></svg>`,
    antigravity: `<svg viewBox="0 0 24 24" fill="currentColor"><path d="M12 3.5 14.2 9l6.3.4-4.9 3.8 1.6 6.1L12 16.2 6.8 19.3 8.4 13.2 3.5 9.4 9.8 9z"/></svg>`,
    grok: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8"><path d="M5 5h6l8 14h-6L5 5zm8 0h6M5 19h6"/></svg>`,
    groq: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8"><circle cx="12" cy="12" r="7"/><path d="M8 12h8"/></svg>`,
    windsurf: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8"><path d="M4 16c3-4 6-6 8-6s5 2 8 6M4 20c3-3 6-4.5 8-4.5S17 17 20 20"/></svg>`,
  };
  return icons[id] || icons.grok;
}

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

function escapeHtml(s: string): string {
  return s.replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c]!));
}

function addedProviders(): ProviderSnapshot[] {
  if (!dash) return [];
  return dash.providers.filter((p) => dash!.catalog.find((c) => c.id === p.id)?.enabled);
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

function renderDash() {
  if (!dash) return;
  const added = addedProviders();
  $("empty").classList.toggle("hidden", added.length > 0);
  $("empty-text").textContent = t("empty");
  $("empty-detect").textContent = t("detect");
  $("empty-add").textContent = t("addManual");
  $("btn-quit").textContent = t("quit");
  document.documentElement.style.setProperty("--accent", accent(selectedId || added[0]?.id || "anthropic"));

  if (!added.find((p) => p.id === selectedId) && added.length) {
    selectedId = dash.primary && added.some((p) => p.id === dash!.primary) ? dash.primary : added[0].id;
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

  $("view-dash").classList.toggle("hidden", view !== "dash");
  $("view-spend").classList.toggle("hidden", view !== "spend");
  $("view-settings").classList.toggle("hidden", view !== "settings");
  $("btn-spend")?.classList.toggle("active", view === "spend");
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
}

function renderSettings() {
  if (!dash) return;
  $("settings-title").textContent = t("settings");
  const cat = dash.catalog;
  const providers = cat
    .map((v) => {
      const keyField = v.needsKey
        ? `<div class="key-row"><input data-key="${v.id}" placeholder="${v.envKey || t("apiKey")}" type="password" /><button data-savekey="${v.id}">${t("save")}</button></div>`
        : `<p class="hint">${escapeHtml(v.hint)}</p>`;
      return `<label class="prov" style="--accent:${accent(v.id)}">
        <input type="checkbox" data-enable="${v.id}" ${v.enabled ? "checked" : ""} />
        <span class="glyph">${glyph(v.id)}</span>
        <span class="prov-name">${escapeHtml(v.name)}</span>
        ${v.detected ? `<span class="dot"></span>` : ""}
      </label>
      ${v.enabled ? keyField : ""}`;
    })
    .join("");
  const enabled = cat.filter((v) => v.enabled);
  const primaryOpts = enabled
    .map((v) => `<option value="${v.id}" ${dash!.primary === v.id ? "selected" : ""}>${escapeHtml(v.name)}</option>`)
    .join("");
  $("settings-body").innerHTML = `
    <h3>${t("providers")}</h3>
    <p class="lede">${t("providersHint")}</p>
    <div class="prov-list">${providers}</div>
    <h3>${t("general")}</h3>
    <label class="row">${t("notifications")}
      <input type="checkbox" id="cfg-notif" ${dash.notifications ? "checked" : ""} />
    </label>
    <p class="lede">${t("notifHint")}</p>
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
    <h3>${t("guide")}</h3>
    <p class="lede">${t("guideHint")}</p>
    ${cat
      .map(
        (v) =>
          `<div class="guide-row"><strong>${escapeHtml(v.name)}</strong><span>${escapeHtml(v.hint)}</span></div>`,
      )
      .join("")}
  `;
}

function renderSpend() {
  if (!dash) return;
  $("spend-title").textContent = t("spend");
  const rows = [...dash.spend].sort((a, b) => b.usd - a.usd);
  if (!rows.length) {
    $("spend-body").innerHTML = `<p class="spend-empty">${t("spendEmpty")}</p>`;
    return;
  }
  $("spend-body").innerHTML = `
    <div class="spend-total">
      <span>${t("spend")}</span>
      <span class="amt">$${dash.spendMonthUsd.toFixed(2)}</span>
    </div>
    ${rows
      .map(
        (r) => `<div class="spend-row">
          <div><div class="name">${escapeHtml(tabName(r.id, r.name))}</div><div class="sub">${escapeHtml(r.label)}</div></div>
          <div>$${r.usd.toFixed(2)}</div>
        </div>`,
      )
      .join("")}
  `;
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
    renderSettings();
    for (const [id, val] of Object.entries(draft)) {
      const el = document.querySelector<HTMLInputElement>(`[data-key="${id}"]`);
      if (el) el.value = val;
    }
  }
  if (view === "spend") renderSpend();
  renderDash();
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

async function persistConfig() {
  if (!dash) return;
  await invokeCmd("set_app_config", {
    cfg: {
      refresh_minutes: Number(($("cfg-interval") as HTMLSelectElement)?.value || dash.refreshMinutes),
      primary: ($("cfg-primary") as HTMLSelectElement)?.value || dash.primary,
      notifications: ($("cfg-notif") as HTMLInputElement)?.checked ?? dash.notifications,
      show_usage_as: dash.showUsageAs,
      reset_times: dash.resetTimes,
      providers: Object.fromEntries(
        dash.catalog.map((v) => [
          v.id,
          { enabled: (document.querySelector(`[data-enable="${v.id}"]`) as HTMLInputElement)?.checked ?? v.enabled },
        ]),
      ),
    },
  });
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
      renderSettings();
      renderDash();
      await fitWindow();
      break;
    case "spend":
      view = "spend";
      renderSpend();
      renderDash();
      await fitWindow();
      break;
    case "back":
      view = "dash";
      renderDash();
      await fitWindow();
      break;
    case "quit":
      await invokeCmd("quit");
      break;
    case "lang-en":
      lang = "en";
      localStorage.setItem("lang", "en");
      renderDash();
      if (view === "settings") renderSettings();
      if (view === "spend") renderSpend();
      break;
    case "lang-es":
      lang = "es";
      localStorage.setItem("lang", "es");
      renderDash();
      if (view === "settings") renderSettings();
      if (view === "spend") renderSpend();
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
      renderDash();
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
    if (el.id?.startsWith("cfg-")) await persistConfig();
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

function previewDashboard(): Dashboard {
  const reset = new Date(Date.now() + 44 * 60 * 1000).toISOString();
  const week = new Date(Date.now() + 75 * 3600 * 1000).toISOString();
  const line = (
    id: string,
    label: string,
    used: number,
    resetsAt: string,
    windowSecs: number,
  ): MetricLine => ({
    kind: "progress",
    id,
    label,
    used,
    limit: 100,
    format: "percent",
    resetsAt,
    resetsInLabel: "",
    windowSecs,
    visible: "always",
  });
  const snap = (
    id: string,
    name: string,
    short: string,
    plan: string,
    a: number,
    b: number,
  ): ProviderSnapshot => ({
    id,
    name,
    short,
    plan,
    connected: true,
    stale: false,
    error: null,
    hint: null,
    updatedAt: new Date().toISOString(),
    lines: [
      line("session", I18N[lang].session, a, reset, 18000),
      line("weekly", I18N[lang].weekly, b, week, 604800),
      ...(id === "cursor"
        ? [line("other", lang === "es" ? "Otros modelos" : "Other models", 10, week, 604800)]
        : []),
    ],
    primaryUtilization: a,
  });
  const catalog = (id: string, name: string, enabled: boolean): VendorInfo => ({
    id,
    name,
    short: id.slice(0, 3).toUpperCase(),
    authKind: "oauth",
    envKey: null,
    hint: "",
    needsKey: false,
    enabled,
    detected: enabled,
  });
  const claude = snap("anthropic", "Claude Code", "CLD", "Max", 88, 12);
  claude.lines.push({ kind: "values", id: "cost_month", label: "Mes", text: "$42.10", visible: "always" });
  return {
    providers: [
      claude,
      snap("openai", "Codex / ChatGPT", "CDX", "Plus", 45, 60),
      snap("cursor", "Cursor", "CUR", "Ultra", 26, 5),
      snap("copilot", "GitHub Copilot", "COP", "Pro", 30, 80),
      snap("antigravity", "Antigravity", "AGY", "Google AI Pro", 55, 25),
    ],
    catalog: [
      catalog("anthropic", "Claude Code", true),
      catalog("openai", "Codex / ChatGPT", true),
      catalog("cursor", "Cursor", true),
      catalog("copilot", "GitHub Copilot", true),
      catalog("antigravity", "Antigravity", true),
      catalog("groq", "Groq", false),
      catalog("windsurf", "Windsurf", false),
    ],
    refreshMinutes: 5,
    primary: "anthropic",
    notifications: true,
    showUsageAs: "used",
    resetTimes: "countdown",
    nextUpdateInSecs: 120,
    spendMonthUsd: 42.1,
    spend: [{ id: "anthropic", name: "Claude Code", label: "Mes", usd: 42.1 }],
    recommendId: "cursor",
    recommendName: "Cursor",
    recommendLeft: 74,
  };
}

main();
