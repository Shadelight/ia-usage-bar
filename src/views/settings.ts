// Settings view: provider list, general options, and the connection guide.

import { $, accent, escapeHtml, glyph, invokeCmd } from "../api";
import type { Dashboard } from "../api";
import { t } from "../i18n";

export function renderSettings(dash: Dashboard | null): void {
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
    .map((v) => `<option value="${v.id}" ${dash.primary === v.id ? "selected" : ""}>${escapeHtml(v.name)}</option>`)
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

export async function persistConfig(dash: Dashboard | null): Promise<void> {
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
