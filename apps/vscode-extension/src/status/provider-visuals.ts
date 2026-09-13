export interface ProviderVisual {
  /** Codicon/Product Icon id, usable as `$(id)` in status bar text or Markdown. */
  icon: string;
  /** 3-letter compact code shown in "compact" display mode. */
  short: string;
}

// Registered in package.json's contributes.icons, backed by
// media/provider-icons/provider-icons.woff (built from the same brand SVGs
// the desktop app ships under src/assets/providers, via `npm run build-icons`).
const KNOWN_VISUALS: Record<string, ProviderVisual> = {
  anthropic: { icon: "ia-anthropic", short: "CLD" },
  openai: { icon: "ia-openai", short: "CDX" },
  cursor: { icon: "ia-cursor", short: "CUR" },
  antigravity: { icon: "ia-antigravity", short: "AGY" },
  opencode_go: { icon: "ia-opencode_go", short: "OCG" },
  openrouter: { icon: "ia-openrouter", short: "ORT" },
  deepseek: { icon: "ia-deepseek", short: "DSK" },
  groq: { icon: "ia-groq", short: "GRQ" },
  kimi: { icon: "ia-kimi", short: "KIM" },
  kilo: { icon: "ia-kilo", short: "KLO" },
  minimax: { icon: "ia-minimax", short: "MMX" },
  grok: { icon: "ia-grok", short: "GRK" },
  copilot: { icon: "ia-copilot", short: "CPT" },
  windsurf: { icon: "ia-windsurf", short: "WSF" },
  zai: { icon: "ia-zai", short: "ZAI" },
  moonshot: { icon: "ia-moonshot", short: "MSH" },
  novita: { icon: "ia-novita", short: "NOV" },
  kiro: { icon: "ia-kiro", short: "KIR" },
  nous: { icon: "ia-nous", short: "NOU" },
};

function fallbackShortCode(name: string): string {
  const letters = name.replace(/[^A-Za-z0-9]/g, "");
  return (letters.slice(0, 3) || "AI").toUpperCase();
}

/** A provider IA Usage doesn't ship a real logo for yet never breaks the
 * status bar: it falls back to a generic dot icon and a derived 3-letter code. */
export function providerVisual(id: string, name: string): ProviderVisual {
  return KNOWN_VISUALS[id] ?? { icon: "circle-filled", short: fallbackShortCode(name) };
}
