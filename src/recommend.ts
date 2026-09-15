// Pure recommendation copy: maps the contextual engine payload
// (`recommendAction/Reason/Scores` from `iausage-core::recommend`) to banner
// text. No DOM here so `tests/recommendation.test.ts` can cover it on Node.

import type { Dashboard, RecommendCandidate, RecommendationSeverity } from "./api.ts";
import { t } from "./i18n.ts";

export interface RecCopy {
  text: string;
  title: string;
  meta: string;
  detailLines: string[];
  severity: RecommendationSeverity;
  selectId: string;
}

function copyResult(
  title: string,
  meta: string,
  selectId: string,
  severity: RecommendationSeverity,
  detailLines: string[] = [],
): RecCopy {
  return { text: meta ? `${title} · ${meta}` : title, title, meta, detailLines, severity, selectId };
}

function conciseProviderName(name: string): string {
  return name.replace(/\s+Code$/i, "").replace(/\s*\/\s*ChatGPT$/i, "");
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
  const action = dash.recommendAction || "";
  if (!action) {
    if (!toName || dash.recommendLeft == null) return null;
    // Payload antiguo sin motor: comportamiento legado.
    const template = toId === selectedId ? t("stallHere") : t("stall");
    const text = fill(template, { name: toName, left: String(Math.round(dash.recommendLeft)) });
    return copyResult(text, "", toId || "", "healthy");
  }
  if (action === "insufficient_data") return null;
  const reason = dash.recommendReason || "";
  const fromId = dash.recommendFrom || dash.primary || selectedId;
  const fromName = providerDisplayName(dash, fromId, fromId);
  const destName = toName;
  const severity = dash.recommendSeverity || "healthy";
  const quota = dash.recommendLimitingQuota;
  const fromCand = candidateById(dash, fromId);
  const exhaustSecs = fromCand?.exhaustInSecs ?? quota?.exhaustsBeforeResetSeconds ?? quota?.resetInSeconds;
  const dur = exhaustSecs != null ? durLabelSecs(exhaustSecs) : "";
  if (action === "switch" && toId && toId === selectedId && fromId !== selectedId) {
    return copyResult(
      fill(t("recAlreadyBest"), { name: conciseProviderName(destName || toId) }),
      dur ? fill(t("recAlreadyBestMeta"), { from: conciseProviderName(fromName), dur }) : t("recHealthyMeta"),
      toId,
      severity === "healthy" ? "healthy" : severity,
    );
  }
  if (dash.recommendSeverity) {
    const shortName = conciseProviderName(fromName);
    let title = fill(t("recHealthyTitle"), { name: shortName });
    let meta = t("recHealthyMeta");
    if (reason === "quota_exhausted") title = fill(t("recExhaustedTitle"), { name: shortName });
    else if (reason === "partial_limited") title = fill(t("recPartialLimitedTitle"), { name: shortName });
    else if (severity === "critical") title = fill(t("recCriticalTitle"), { name: shortName });
    else if (reason === "projected_exhaustion") title = fill(t("recWarningTitle"), { name: shortName });
    else if (reason === "auth_problem" || reason === "current_unavailable") title = fill(t("recAuthTitle"), { name: shortName });
    else if (reason === "service_degraded" || reason === "service_outage") title = fill(t("recServiceTitle"), { name: shortName });

    if (reason === "partial_limited") {
      meta = fill(t("recPartialLimitedMeta"), {
        quota: quota?.label || t("someModels"),
      });
    } else if (quota && severity === "critical") {
      const reset = quota.resetInSeconds == null ? "" : durLabelSecs(quota.resetInSeconds);
      meta = fill(t(reset ? "recQuotaMeta" : "recQuotaMetaNoReset"), {
        quota: quota.label,
        used: String(Math.round(quota.usedPercent)),
        reset,
      });
    } else if (quota && reason === "projected_exhaustion") {
      meta = fill(t("recProjectedMeta"), {
        dur: durLabelSecs(quota.exhaustsBeforeResetSeconds ?? 0),
      });
    } else if (reason === "auth_problem" || reason === "current_unavailable") {
      meta = t("recAuthMeta");
    } else if (reason === "service_degraded" || reason === "service_outage") {
      meta = t("recServiceMeta");
    }

    const detailLines: string[] = [];
    if (quota?.expectedUsedPercent != null) {
      detailLines.push(fill(t("recExpectedDetail"), { value: String(Math.round(quota.expectedUsedPercent)) }));
    }
    if (quota) detailLines.push(fill(t("recActualDetail"), { value: String(Math.round(quota.usedPercent)) }));
    if (quota?.deltaPercent != null && quota.deltaPercent > 0) {
      detailLines.push(fill(t("recDeltaDetail"), { value: String(Math.round(quota.deltaPercent)) }));
    }
    detailLines.push(fill(t("recConfidenceDetail"), { value: t(recConfidenceKey(dash.recommendConfidence)) }));
    if (action === "switch" && toId && toId !== fromId && destName) {
      detailLines.push(fill(t("recAlternative"), { name: conciseProviderName(destName) }));
    } else if (severity !== "healthy") {
      detailLines.push(t("recNoAlternative"));
    }
    return copyResult(title, meta, toId || fromId, severity, detailLines);
  }

  if (!toName || dash.recommendLeft == null) return null;
  const toCand = candidateById(dash, toId);

  if (action === "balanced") {
    return copyResult(t("recBalanced"), "", toId || "", "healthy");
  }
  if (reason === "reserve_no_history") {
    // El destino (Stay) es el actual; la novedad es la reserva 100% vacía.
    const reserve = dash.recommendScores?.find((c) => c.isReserve && !c.excluded);
    const name = reserve?.name ?? destName;
    return copyResult(fill(t("recReserve"), { name }), "", reserve?.id ?? (toId || ""), "healthy");
  }
  if (action === "stay" && (reason === "sustainable" || reason === "current_unavailable")) {
    return copyResult(fill(t("recStay"), { name: providerDisplayName(dash, toId, destName) }), "", toId || "", "healthy");
  }
  if (reason === "at_risk") {
    const altName = action === "stay" && toId === fromId
      ? (bestAltName(dash, fromId) ?? destName)
      : destName;
    const altId = action === "stay" && toId === fromId
      ? (bestAltId(dash, fromId) ?? toId ?? "")
      : (toId || "");
    return copyResult(fill(t("recStayAtRisk"), { from: fromName, to: altName }), "", altId, "warning");
  }
  if (action === "stay" && (reason === "critical_short" || reason === "exhausts_before_reset")) {
    // El actual se agota pero ninguna alternativa supera la histéresis:
    // avisar sin ordenar un cambio que el motor no respalda.
    if (!dur) return copyResult(fill(t("recStayAtRisk"), { from: fromName, to: destName }), "", toId || "", "warning");
    return copyResult(fill(t("recStayExhausts"), { from: fromName, dur }), "", toId || "", "warning");
  }
  if (action === "switch" && reason === "critical_short" && dur) {
    return copyResult(fill(t("recCritical"), { name: destName, from: fromName, dur }), "", toId || "", "critical");
  }
  if (action === "switch" && reason === "exhausts_before_reset" && dur) {
    return copyResult(fill(t("recSwitch"), { name: destName, from: fromName, dur }), "", toId || "", "warning");
  }
  // Switch por margen (current sostenible pero alternativa muy superior)
  // o sin dato de agotamiento: hablar de margen, nunca inventar tiempos.
  const gap = fromCand && toCand ? Math.round(toCand.score - fromCand.score) : NaN;
  if (Number.isFinite(gap) && gap > 0) {
    return copyResult(fill(t("recSwitchMargin"), { name: destName, gap: String(gap), from: fromName }), "", toId || "", "healthy");
  }
  return copyResult(fill(t("recStayAtRisk"), { from: fromName, to: destName }), "", toId || "", "warning");
}

/** Prefijo visual del banner según la acción: ✓ quedarse, ↗ cambiar, = equilibrado. */
export function recPrefix(dash: Dashboard): string {
  if (dash.recommendSeverity === "critical") return "⛔ ";
  if (dash.recommendSeverity === "warning") return "⚠ ";
  if (dash.recommendSeverity === "healthy") return "✓ ";
  switch (dash.recommendAction) {
    case "switch": return dash.recommendReason === "critical_short" ? "⛔ " : "↗ ";
    case "balanced": return "= ";
    case "stay": return dash.recommendReason === "at_risk" || dash.recommendReason === "exhausts_before_reset" || dash.recommendReason === "critical_short" ? "⚠ " : "✓ ";
    default: return "";
  }
}
