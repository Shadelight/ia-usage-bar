// Pure recommendation copy: maps the contextual engine payload
// (`recommendAction/Reason/Scores` from `iausage-core::recommend`) to banner
// text. No DOM here so `tests/recommendation.test.ts` can cover it on Node.

import type { Dashboard, RecommendCandidate } from "./api.ts";
import { t } from "./i18n.ts";

export interface RecCopy {
  text: string;
  selectId: string;
}

export function durLabelSecs(secs: number): string {
  if (!Number.isFinite(secs) || secs < 0) return "";
  const days = Math.floor(secs / 86_400);
  const hours = Math.floor((secs % 86_400) / 3_600);
  const minutes = Math.round((secs % 3_600) / 60);
  if (days >= 2) return `${days}d ${hours}h`;
  if (days === 1) return `1d ${hours}h`;
  if (hours >= 1) return `${hours}h ${minutes}m`;
  return `${Math.max(minutes, 1)} min`;
}

export function providerDisplayName(dash: Dashboard, id: string | null | undefined, fallback: string): string {
  if (!id) return fallback;
  const vendor = dash.catalog.find((v) => v.id === id);
  if (vendor) return vendor.name;
  const snap = dash.providers.find((p) => p.id === id);
  return snap?.name ?? fallback;
}

export function candidateById(dash: Dashboard, id: string | null | undefined): RecommendCandidate | undefined {
  if (!id) return undefined;
  return dash.recommendScores?.find((c) => c.id === id);
}

export function recConfidenceKey(confidence: number | null | undefined): string {
  if (confidence == null) return "recConfidenceMid";
  if (confidence >= 0.75) return "recConfidenceHigh";
  if (confidence >= 0.5) return "recConfidenceMid";
  return "recConfidenceLow";
}

/** Tooltip "¿Por qué?" con el breakdown de los 3 mejores candidatos. */
export function recWhyTitle(dash: Dashboard): string {
  const scores = dash.recommendScores?.slice(0, 3) ?? [];
  if (!scores.length) return "";
  const lines = scores.map((c) => {
    const left = c.displayLeft != null ? `${Math.round(c.displayLeft)}%` : "—";
    const verdict = !c.hasData || c.isReserve
      ? t("recNoHistory")
      : c.sustainable === false
        ? t("recExhaustsBefore")
        : t("recReachesReset");
    const reset = c.resetInSecs != null ? ` · ${t("resetsIn")} ${durLabelSecs(c.resetInSecs)}` : "";
    return `${c.name}: ${left} · ${verdict}${reset}`;
  });
  lines.push(`${t("recWhy")} ${t(recConfidenceKey(dash.recommendConfidence))}`);
  return lines.join("\n");
}

function fill(template: string, vars: Record<string, string>): string {
  let out = template;
  for (const [k, v] of Object.entries(vars)) out = out.split(`{${k}}`).join(v);
  return out;
}

export function bestAltId(dash: Dashboard, excludeId: string): string | undefined {
  return dash.recommendScores?.find((c) => c.id !== excludeId && c.hasData && !c.excluded)?.id;
}

export function bestAltName(dash: Dashboard, excludeId: string): string | undefined {
  return dash.recommendScores?.find((c) => c.id !== excludeId && c.hasData && !c.excluded)?.name;
}

/**
 * Copia contextual del banner. Null cuando no hay recomendación que mostrar
 * (legado: usa stall/stallHere si el payload no trae el motor).
 */
export function recCopy(dash: Dashboard, selectedId: string): RecCopy | null {
  const toId = dash.recommendId;
  const toName = dash.recommendName ?? "";
  if (!toName || dash.recommendLeft == null) return null;
  const action = dash.recommendAction || "";
  if (!action) {
    // Payload antiguo sin motor: comportamiento legado.
    const template = toId === selectedId ? t("stallHere") : t("stall");
    return {
      text: fill(template, { name: toName, left: String(Math.round(dash.recommendLeft)) }),
      selectId: toId || "",
    };
  }
  if (action === "insufficient_data") return null;
  const reason = dash.recommendReason || "";
  const fromId = dash.recommendFrom || dash.primary || selectedId;
  const fromName = providerDisplayName(dash, fromId, fromId);
  const destName = toName;
  const fromCand = candidateById(dash, fromId);
  const toCand = candidateById(dash, toId);
  const exhaustSecs = fromCand?.exhaustInSecs;
  const dur = exhaustSecs != null ? durLabelSecs(exhaustSecs) : "";

  if (action === "balanced") {
    return { text: t("recBalanced"), selectId: toId || "" };
  }
  if (reason === "reserve_no_history") {
    // El destino (Stay) es el actual; la novedad es la reserva 100% vacía.
    const reserve = dash.recommendScores?.find((c) => c.isReserve && !c.excluded);
    const name = reserve?.name ?? destName;
    return { text: fill(t("recReserve"), { name }), selectId: reserve?.id ?? (toId || "") };
  }
  if (action === "stay" && (reason === "sustainable" || reason === "current_unavailable")) {
    return { text: fill(t("recStay"), { name: providerDisplayName(dash, toId, destName) }), selectId: toId || "" };
  }
  if (reason === "at_risk") {
    const altName = action === "stay" && toId === fromId
      ? (bestAltName(dash, fromId) ?? destName)
      : destName;
    const altId = action === "stay" && toId === fromId
      ? (bestAltId(dash, fromId) ?? toId ?? "")
      : (toId || "");
    return { text: fill(t("recStayAtRisk"), { from: fromName, to: altName }), selectId: altId };
  }
  if (action === "stay" && (reason === "critical_short" || reason === "exhausts_before_reset")) {
    // El actual se agota pero ninguna alternativa supera la histéresis:
    // avisar sin ordenar un cambio que el motor no respalda.
    if (!dur) return { text: fill(t("recStayAtRisk"), { from: fromName, to: destName }), selectId: toId || "" };
    return { text: fill(t("recStayExhausts"), { from: fromName, dur }), selectId: toId || "" };
  }
  if (action === "switch" && reason === "critical_short" && dur) {
    return { text: fill(t("recCritical"), { name: destName, from: fromName, dur }), selectId: toId || "" };
  }
  if (action === "switch" && reason === "exhausts_before_reset" && dur) {
    return { text: fill(t("recSwitch"), { name: destName, from: fromName, dur }), selectId: toId || "" };
  }
  // Switch por margen (current sostenible pero alternativa muy superior)
  // o sin dato de agotamiento: hablar de margen, nunca inventar tiempos.
  const gap = fromCand && toCand ? Math.round(toCand.score - fromCand.score) : NaN;
  if (Number.isFinite(gap) && gap > 0) {
    return {
      text: fill(t("recSwitchMargin"), { name: destName, gap: String(gap), from: fromName }),
      selectId: toId || "",
    };
  }
  return { text: fill(t("recStayAtRisk"), { from: fromName, to: destName }), selectId: toId || "" };
}

/** Prefijo visual del banner según la acción: ✓ quedarse, ↗ cambiar, = equilibrado. */
export function recPrefix(dash: Dashboard): string {
  switch (dash.recommendAction) {
    case "switch": return dash.recommendReason === "critical_short" ? "⛔ " : "↗ ";
    case "balanced": return "= ";
    case "stay": return dash.recommendReason === "at_risk" || dash.recommendReason === "exhausts_before_reset" || dash.recommendReason === "critical_short" ? "⚠ " : "✓ ";
    default: return "";
  }
}
