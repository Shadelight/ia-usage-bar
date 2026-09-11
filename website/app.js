const COPY = {
  en: {
    navFeatures: "Features",
    navProviders: "Providers",
    navPrivacy: "Privacy",
    navGithub: "GitHub",
    navDownload: "Download",
    heroTitle: "All your AI limits.<br>One <span>Windows</span> app.",
    heroSubtitle:
      "Monitor session quotas, weekly resets, spend, and provider status for the AI tools you actually use — in one tray app.",
    ctaPrimary: "Download for Windows",
    ctaSecondary: "View on GitHub",
    finePrint: "Free · Open source · Windows 10/11",
    providersEyebrow: "Providers",
    providersTitle: "One app, every provider you pay for",
    providersSubtitle:
      "Enable only what you use. IA Usage Bar never disables a provider for you.",
    providersMore: "+ more via API key or local session",
    limitsKicker: "Limits & resets",
    limitsTitle: "Session, weekly, monthly — all in one glance",
    limitsSubtitle:
      "Used vs. remaining percentages, exact and relative reset countdowns, and pace warnings when you're burning faster than the window allows.",
    limitCard1t: "Session quota",
    limitCard1p: "Live used/remaining for the current session window, per provider.",
    limitCard2t: "Weekly & monthly",
    limitCard2p: "Rolling-window and calendar quotas with exact reset time and countdown.",
    limitCard3t: "Stale-data handling",
    limitCard3p: "If a provider is unreachable, the last good numbers stay on screen — flagged, not hidden.",
    calloutBadge: "Switch",
    calloutText: "<b>Cursor</b> has the most headroom right now (74%) — IA Usage Bar flags it before you hit a wall on your primary provider.",
    compactKicker: "Compact mode",
    compactTitle: "Small window, same signal",
    compactP1:
      "Shrinks to the selected provider's primary quota and hides secondary detail. Pin keeps it always-on-top.",
    compactP2:
      "Both states persist across restarts, and are mirrored in the tray menu.",
    spendKicker: "Spend & credits",
    spendTitle: "Know your usage and estimated cost",
    spendP1:
      "See provider-reported spend where a provider exposes it, and a local cost estimate from usage logs where it doesn't — clearly labeled, never mixed together.",
    spendLi1tag: "Provider-reported",
    spendLi1: "figures straight from the provider's own usage endpoint",
    spendLi2tag: "Estimated",
    spendLi2: "local cost equivalent calculated from usage logs, not a bill",
    statusKicker: "Status & notifications",
    statusTitle: "Structured status, not a generic error",
    statusSubtitle:
      "Needs login, needs permission, service unavailable, rate limited — each provider reports what's actually wrong. Notification thresholds are configurable; alerts fire once per threshold per window, plus a reset notice.",
    statusCard1t: "Needs login",
    statusCard1p: "Sign in to the provider's app/CLI, then Detect or Refresh.",
    statusCard2t: "Needs permission",
    statusCard2p: "Credential authenticates but can't read usage — check the plan.",
    statusCard3t: "Rate limited / unavailable",
    statusCard3p: "IA Usage Bar couldn't fetch fresh data right now. It keeps the last good snapshot and shows the cause when it can tell.",
    privacyKicker: "Privacy & security",
    privacyTitle: "Your credentials stay on your machine",
    privacySubtitle:
      "IA Usage Bar reuses sessions and credentials that already exist locally when a provider allows it — OAuth/CLI login, device flows, local session files — instead of asking you to re-authenticate.",
    privacyLi1: "Manually entered API keys are stored in Windows Credential Manager, never in plaintext config.",
    privacyLi2: "No passwords are ever stored by the app.",
    privacyLi3: "Processing happens locally, except the network calls each enabled provider's own usage endpoint needs.",
    privacyLi4: "No telemetry, no analytics.",
    privacyLink: "Full policy in SECURITY.md →",
    privacyRow1k: "Provider session",
    privacyRow1v: "Managed by the original CLI/app, read locally",
    privacyRow2k: "Manual API keys",
    privacyRow2v: "Windows Credential Manager",
    privacyRow3k: "Configuration",
    privacyRow3v: "Local only",
    privacyRow4k: "Telemetry",
    privacyRow4vNone: "None",
    ossKicker: "Open source",
    ossTitle: "MIT licensed, inspect everything",
    ossSubtitle: "No black box. Read the code, file an issue, or send a PR.",
    ossCard1t: "Source",
    ossCard1p: "Full history and issue tracker on GitHub.",
    ossCard2t: "Contributing",
    ossCard2p: "Setup, coding conventions, and PR checklist.",
    ossCard3t: "Security",
    ossCard3p: "Vulnerability reporting and credential-handling policy.",
    finalTitle: "Stop guessing your quota.",
    finalSubtitle: "One tray app, every provider, always current.",
    finalCta: "Download IA Usage Bar for Windows",
    footerNote: "Not affiliated with Anthropic, OpenAI, Cursor, or any listed provider.",
  },
  es: {
    navFeatures: "Funciones",
    navProviders: "Proveedores",
    navPrivacy: "Privacidad",
    navGithub: "GitHub",
    navDownload: "Descargar",
    heroTitle: "Todos tus límites de IA.<br>Una sola app de <span>Windows</span>.",
    heroSubtitle:
      "Monitoriza cuotas de sesión, reinicios semanales, gasto y estado de las herramientas de IA que realmente usas — en una sola ventana residente.",
    ctaPrimary: "Descargar para Windows",
    ctaSecondary: "Ver en GitHub",
    finePrint: "Gratis · Open source · Windows 10/11",
    providersEyebrow: "Proveedores",
    providersTitle: "Una app, todos los proveedores que pagas",
    providersSubtitle:
      "Activa solo los que usas. IA Usage Bar nunca desactiva un proveedor por ti.",
    providersMore: "+ más vía clave API o sesión local",
    limitsKicker: "Límites y reinicios",
    limitsTitle: "Sesión, semanal, mensual — todo de un vistazo",
    limitsSubtitle:
      "Porcentaje usado vs. disponible, cuenta atrás exacta y relativa del reinicio, y avisos de ritmo cuando consumes más rápido de lo que da la ventana.",
    limitCard1t: "Cuota de sesión",
    limitCard1p: "Usado/disponible en vivo para la ventana de sesión actual, por proveedor.",
    limitCard2t: "Semanal y mensual",
    limitCard2p: "Cuotas de ventana móvil y calendario con hora exacta de reinicio y cuenta atrás.",
    limitCard3t: "Manejo de datos obsoletos",
    limitCard3p: "Si un proveedor no responde, se mantienen los últimos datos válidos — marcados, no ocultos.",
    calloutBadge: "Cambia",
    calloutText: "<b>Cursor</b> tiene el mayor margen ahora mismo (74%) — IA Usage Bar te avisa antes de quedarte sin cuota en tu proveedor principal.",
    compactKicker: "Modo compacto",
    compactTitle: "Ventana pequeña, misma señal",
    compactP1:
      "Se reduce a la cuota principal del proveedor seleccionado y oculta el detalle secundario. Pin la mantiene siempre visible.",
    compactP2:
      "Ambos estados persisten entre reinicios y se reflejan en el menú de la bandeja.",
    spendKicker: "Gasto y créditos",
    spendTitle: "Entiende tu uso y coste estimado",
    spendP1:
      "Consulta el gasto reportado por el proveedor cuando lo expone, y una estimación local calculada a partir de los logs de uso cuando no — siempre etiquetado, nunca mezclado.",
    spendLi1tag: "Reportado por el proveedor",
    spendLi1: "cifras directas del propio endpoint de uso del proveedor",
    spendLi2tag: "Estimado",
    spendLi2: "coste equivalente calculado localmente a partir de los logs de uso, no una factura",
    statusKicker: "Estado y notificaciones",
    statusTitle: "Estado estructurado, no un error genérico",
    statusSubtitle:
      "Necesita login, necesita permiso, servicio no disponible, límite de tasa — cada proveedor reporta lo que realmente falla. Los umbrales de notificación son configurables; las alertas saltan una vez por umbral y ventana, más un aviso al reiniciarse.",
    statusCard1t: "Necesita login",
    statusCard1p: "Inicia sesión en la app/CLI del proveedor y usa Detectar o Actualizar.",
    statusCard2t: "Necesita permiso",
    statusCard2p: "La credencial autentica pero no puede leer el uso — revisa el plan.",
    statusCard3t: "Límite de tasa / no disponible",
    statusCard3p: "IA Usage Bar no pudo obtener datos frescos ahora. Conserva el último dato válido y muestra la causa cuando puede determinarla.",
    privacyKicker: "Privacidad y seguridad",
    privacyTitle: "Tus credenciales se quedan en tu equipo",
    privacySubtitle:
      "IA Usage Bar reutiliza sesiones y credenciales que ya existen localmente cuando el proveedor lo permite — OAuth/CLI, flujos de dispositivo, archivos de sesión locales — en vez de pedirte que vuelvas a autenticarte.",
    privacyLi1: "Las claves API añadidas manualmente se guardan en Windows Credential Manager, nunca en texto plano.",
    privacyLi2: "La app nunca almacena contraseñas.",
    privacyLi3: "El procesamiento es local, salvo las llamadas que cada proveedor activo necesita a su propio endpoint de uso.",
    privacyLi4: "Sin telemetría, sin analítica.",
    privacyLink: "Política completa en SECURITY.md →",
    privacyRow1k: "Sesión del proveedor",
    privacyRow1v: "La gestiona la app/CLI original, leída localmente",
    privacyRow2k: "Claves API manuales",
    privacyRow2v: "Windows Credential Manager",
    privacyRow3k: "Configuración",
    privacyRow3v: "Solo local",
    privacyRow4k: "Telemetría",
    privacyRow4vNone: "Ninguna",
    ossKicker: "Open source",
    ossTitle: "Licencia MIT, inspecciona todo",
    ossSubtitle: "Sin caja negra. Lee el código, abre un issue o envía un PR.",
    ossCard1t: "Código",
    ossCard1p: "Historial completo e issues en GitHub.",
    ossCard2t: "Contribuir",
    ossCard2p: "Setup, convenciones de código y checklist de PR.",
    ossCard3t: "Seguridad",
    ossCard3p: "Política de reporte de vulnerabilidades y manejo de credenciales.",
    finalTitle: "Deja de adivinar tu cuota.",
    finalSubtitle: "Una app en la bandeja, todos los proveedores, siempre al día.",
    finalCta: "Descargar IA Usage Bar para Windows",
    footerNote: "No afiliado con Anthropic, OpenAI, Cursor ni ningún proveedor listado.",
  },
};

function applyCopy(lang) {
  const dict = COPY[lang] || COPY.en;
  document.querySelectorAll("[data-i18n]").forEach((el) => {
    const key = el.getAttribute("data-i18n");
    if (dict[key] !== undefined) el.innerHTML = dict[key];
  });
  document.documentElement.lang = lang;
  document.querySelectorAll("[data-lang-btn]").forEach((btn) => {
    btn.classList.toggle("active", btn.getAttribute("data-lang-btn") === lang);
  });
  try {
    localStorage.setItem("iausage-lang", lang);
  } catch (_) {}
}

function initLang() {
  let lang = "es";
  try {
    const saved = localStorage.getItem("iausage-lang");
    if (saved === "en" || saved === "es") lang = saved;
    else if (!navigator.language.toLowerCase().startsWith("es")) lang = "en";
  } catch (_) {
    if (!navigator.language.toLowerCase().startsWith("es")) lang = "en";
  }
  applyCopy(lang);
  document.querySelectorAll("[data-lang-btn]").forEach((btn) => {
    btn.addEventListener("click", () => applyCopy(btn.getAttribute("data-lang-btn")));
  });
}

function initTheme() {
  const root = document.documentElement;
  const btn = document.querySelector("[data-theme-btn]");
  let saved = null;
  try {
    saved = localStorage.getItem("iausage-theme");
  } catch (_) {}
  if (saved) root.setAttribute("data-theme", saved);
  if (btn) {
    btn.addEventListener("click", () => {
      const current = root.getAttribute("data-theme") || (matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light");
      const next = current === "dark" ? "light" : "dark";
      root.setAttribute("data-theme", next);
      try {
        localStorage.setItem("iausage-theme", next);
      } catch (_) {}
    });
  }
}

document.addEventListener("DOMContentLoaded", () => {
  initLang();
  initTheme();
  const yearEl = document.querySelector("[data-year]");
  if (yearEl) yearEl.textContent = String(new Date().getFullYear());
});
