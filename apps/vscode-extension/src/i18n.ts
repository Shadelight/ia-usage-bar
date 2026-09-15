export type Lang = "es" | "en";

type Dict = Record<string, string>;

const es: Dict = {
  "common.ago": "hace {age}",
  "common.justNow": "un momento",
  "menu.title": "IA Usage",
  "menu.customize": "$(settings-gear) Ajustes rápidos",
  "menu.chooseProviders": "$(list-selection) Elegir proveedores…",
  "menu.refresh": "$(refresh) Actualizar ahora",
  "menu.refreshDesc": "Consulta IA Usage una vez",
  "menu.reconnect": "$(plug) Reconectar CLI",
  "menu.reconnectDesc": "Reinicia el stream local",
  "menu.fullSettings": "$(gear) Ajustes completos",

  "providers.pickTitle": "IA Usage: elegir proveedores en la barra",
  "providers.noneAvailable": "IA Usage: aún no hay proveedores activos que mostrar. Actívalos primero en IA Usage Desktop.",
  "providers.reorderTitle": "IA Usage: reordenar proveedores (elegí en el orden deseado)",

  "detail.chooseMetric": "$(graph) Elegir métrica principal",
  "detail.toggleVisible": "$(eye) Mostrar/ocultar en barra",
  "detail.resetsIn": "reinicia en {time}",

  "quickSettings.title": "IA Usage · Ajustes rápidos",
  "quickSettings.providers": "$(list-selection) Proveedores visibles",
  "quickSettings.appearance": "$(symbol-color) Apariencia",
  "quickSettings.primaryMetric": "$(graph) Métrica principal",
  "quickSettings.percentage": "$(percentage) Mostrar usado/restante",
  "quickSettings.reset": "$(clock) Mostrar reset",
  "quickSettings.tooltip": "$(eye) Información del tooltip",
  "quickSettings.fullSettings": "$(settings-gear) Abrir ajustes completos",

  "appearance.title": "IA Usage · Apariencia",
  "appearance.density": "Densidad",
  "appearance.densityMinimal": "Minimal — icono + porcentaje",
  "appearance.densityCompact": "Compact — icono + código + porcentaje",
  "appearance.densityFull": "Full — icono + nombre completo + porcentaje",
  "appearance.icons": "Iconos de proveedor",
  "appearance.iconsOn": "Mostrar",
  "appearance.iconsOff": "Ocultar",
  "appearance.stale": "Indicador de datos antiguos",
  "appearance.staleOn": "Mostrar",
  "appearance.staleOff": "Ocultar",

  "percentage.title": "IA Usage · Qué porcentaje mostrar",
  "percentage.used": "Usado",
  "percentage.usedExample": "CDX 21%",
  "percentage.remaining": "Restante",
  "percentage.remainingExample": "CDX 79% libre",
  "percentage.available": "Restante",
  "percentage.availableExample": "CDX 79% libre",

  "reset.title": "IA Usage · Reset en la barra",
  "reset.hidden": "Oculto",
  "reset.shown": "Mostrar",
  "reset.shownExample": "Ejemplo: CDX 21% (39m)",

  "tooltip.info.title": "IA Usage · Información del tooltip",
  "tooltip.info.primary": "Solo lo relevante",
  "tooltip.info.primaryDesc": "Métrica principal + secundarias con uso",
  "tooltip.info.all": "Todas las métricas",
  "tooltip.info.allDesc": "Incluye métricas en 0%",

  "quota.weekly": "Semanal",
  "quota.fiveHour": "5 horas",
  "quota.session": "Sesión",

  "primaryMetric.chooseProvider": "IA Usage: elegir proveedor",
  "primaryMetric.chooseQuota": "IA Usage: métrica principal de {provider}",
  "primaryMetric.usedReset": "{used}% usado · {resets}",

  "tooltip.updatedAgo": "Actualizado hace {age}",
  "tooltip.updatedNow": "Actualizado ahora",
  "tooltip.stale": "Datos antiguos",
  "tooltip.usedAvailable": "{used}% usado · {available}% restante",
  "tooltip.resetsIn": "Reinicia en {time}",
  "tooltip.credits": "Créditos: {balance}",
  "tooltip.resetCredits": "Restablecimientos disponibles: {n}",
  "tooltip.moreProviders": "+{n} proveedores más configurados",
  "recommendation.critical": "{name} casi agotado",
  "recommendation.exhausted": "{name} no está disponible",
  "recommendation.warning": "{name} va demasiado rápido",
  "recommendation.healthy": "{name} tiene margen suficiente",
  "recommendation.quota": "{quota} · {used}% usado · reinicia en {reset}",
  "recommendation.quotaNoReset": "{quota} · {used}% usado",
  "recommendation.projected": "Se agotará ~{time} antes del reset",
  "recommendation.reachesReset": "Al ritmo actual llegará al próximo reset",
  "recommendation.alternative": "{name} es la mejor alternativa",
  "recommendation.details": "Ver explicación",
  "recommendation.expected": "Esperado ahora: {value}%",
  "recommendation.actual": "Uso actual: {value}%",
  "recommendation.delta": "Diferencia: +{value} pts",

  "error.cliNotFound": "IA Usage: CLI no encontrado. Configura iaUsage.cliPath, añade iausage al PATH o instala IA Usage Desktop.",
  "error.cliStartFailed": "IA Usage: no se pudo iniciar el CLI ({error}).",
  "error.streamEnded": "IA Usage: el stream terminó ({code}); reintentando.",
  "error.invalidJson": "IA Usage: el CLI emitió una línea JSON inválida.",
  "error.refreshFailed": "IA Usage: no se pudo actualizar ({error}).",
  "error.configureCli": "Configurar CLI",
  "error.waiting": "IA Usage: esperando al CLI",
  "statusbar.free": "libre",
};

const en: Dict = {
  "common.ago": "{age} ago",
  "common.justNow": "just now",
  "menu.title": "IA Usage",
  "menu.customize": "$(settings-gear) Quick settings",
  "menu.chooseProviders": "$(list-selection) Choose providers…",
  "menu.refresh": "$(refresh) Refresh now",
  "menu.refreshDesc": "Query IA Usage once",
  "menu.reconnect": "$(plug) Reconnect CLI",
  "menu.reconnectDesc": "Restarts the local stream",
  "menu.fullSettings": "$(gear) Open full settings",

  "providers.pickTitle": "IA Usage: choose status bar providers",
  "providers.noneAvailable": "IA Usage: no active providers to show yet. Enable one in IA Usage Desktop first.",
  "providers.reorderTitle": "IA Usage: reorder providers (pick in the order you want)",

  "detail.chooseMetric": "$(graph) Choose primary metric",
  "detail.toggleVisible": "$(eye) Show/hide in status bar",
  "detail.resetsIn": "resets in {time}",

  "quickSettings.title": "IA Usage · Quick settings",
  "quickSettings.providers": "$(list-selection) Visible providers",
  "quickSettings.appearance": "$(symbol-color) Appearance",
  "quickSettings.primaryMetric": "$(graph) Primary metric",
  "quickSettings.percentage": "$(percentage) Show used/remaining",
  "quickSettings.reset": "$(clock) Show reset",
  "quickSettings.tooltip": "$(eye) Tooltip information",
  "quickSettings.fullSettings": "$(settings-gear) Open full settings",

  "appearance.title": "IA Usage · Appearance",
  "appearance.density": "Density",
  "appearance.densityMinimal": "Minimal — icon + percentage",
  "appearance.densityCompact": "Compact — icon + short code + percentage",
  "appearance.densityFull": "Full — icon + full name + percentage",
  "appearance.icons": "Provider icons",
  "appearance.iconsOn": "Show",
  "appearance.iconsOff": "Hide",
  "appearance.stale": "Stale data indicator",
  "appearance.staleOn": "Show",
  "appearance.staleOff": "Hide",

  "percentage.title": "IA Usage · Which percentage to show",
  "percentage.used": "Used",
  "percentage.usedExample": "CDX 21%",
  "percentage.remaining": "Remaining",
  "percentage.remainingExample": "CDX 79% free",
  "percentage.available": "Remaining",
  "percentage.availableExample": "CDX 79% free",

  "reset.title": "IA Usage · Reset in the status bar",
  "reset.hidden": "Hidden",
  "reset.shown": "Show",
  "reset.shownExample": "Example: CDX 21% (39m)",

  "tooltip.info.title": "IA Usage · Tooltip information",
  "tooltip.info.primary": "Relevant only",
  "tooltip.info.primaryDesc": "Primary metric + secondary metrics in use",
  "tooltip.info.all": "All metrics",
  "tooltip.info.allDesc": "Includes metrics at 0%",

  "quota.weekly": "Weekly",
  "quota.fiveHour": "5 hours",
  "quota.session": "Session",

  "primaryMetric.chooseProvider": "IA Usage: choose provider",
  "primaryMetric.chooseQuota": "IA Usage: primary metric for {provider}",
  "primaryMetric.usedReset": "{used}% used · {resets}",

  "tooltip.updatedAgo": "Updated {age} ago",
  "tooltip.updatedNow": "Updated now",
  "tooltip.stale": "Stale data",
  "tooltip.usedAvailable": "{used}% used · {available}% remaining",
  "tooltip.resetsIn": "Resets in {time}",
  "tooltip.credits": "Credits: {balance}",
  "tooltip.resetCredits": "Available resets: {n}",
  "tooltip.moreProviders": "+{n} more configured providers",
  "recommendation.critical": "{name} is almost out",
  "recommendation.exhausted": "{name} is unavailable",
  "recommendation.warning": "{name} is being used too quickly",
  "recommendation.healthy": "{name} has enough headroom",
  "recommendation.quota": "{quota} · {used}% used · resets in {reset}",
  "recommendation.quotaNoReset": "{quota} · {used}% used",
  "recommendation.projected": "Will run out ~{time} before reset",
  "recommendation.reachesReset": "At the current pace it will reach the next reset",
  "recommendation.alternative": "{name} is the best alternative",
  "recommendation.details": "View explanation",
  "recommendation.expected": "Expected by now: {value}%",
  "recommendation.actual": "Current usage: {value}%",
  "recommendation.delta": "Difference: +{value} pts",

  "error.cliNotFound": "IA Usage: CLI not found. Set iaUsage.cliPath, add iausage to PATH, or install IA Usage Desktop.",
  "error.cliStartFailed": "IA Usage: could not start the CLI ({error}).",
  "error.streamEnded": "IA Usage: the stream ended ({code}); retrying.",
  "error.invalidJson": "IA Usage: the CLI emitted an invalid JSON line.",
  "error.refreshFailed": "IA Usage: could not refresh ({error}).",
  "error.configureCli": "Configure CLI",
  "error.waiting": "IA Usage: waiting for the CLI",
  "statusbar.free": "free",
};

const dicts: Record<Lang, Dict> = { es, en };

/** Set once at activation from vscode.env.language (see extension.ts). Kept
 * out of this module so i18n.ts has zero runtime dependency on the `vscode`
 * module and can be imported from pure logic (format.ts) and unit-tested
 * directly with plain Node. */
let current: Lang = "en";

export function setLang(lang: Lang): void { current = lang; }
export function currentLang(): Lang { return current; }

export function t(key: string, vars?: Record<string, string | number>): string {
  const template = dicts[current][key] ?? dicts.en[key] ?? key;
  if (!vars) return template;
  return Object.entries(vars).reduce((text, [name, value]) => text.replaceAll(`{${name}}`, String(value)), template);
}
