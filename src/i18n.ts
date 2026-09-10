// UI strings + the current language (a live-bound export; mutate via setLang).

export const I18N: Record<string, Record<string, string>> = {
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

export let lang: "es" | "en" = localStorage.getItem("lang") === "en" ? "en" : "es";

export function setLang(l: "es" | "en"): void {
  lang = l;
  localStorage.setItem("lang", l);
}

export function t(k: string): string {
  return I18N[lang][k] ?? k;
}
