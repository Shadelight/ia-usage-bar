import anthropicIcon from "./assets/providers/anthropic.svg";
import antigravityIcon from "./assets/providers/antigravity.svg";
import copilotIcon from "./assets/providers/copilot.svg";
import cursorIcon from "./assets/providers/cursor.svg";
import deepseekIcon from "./assets/providers/deepseek.svg";
import grokIcon from "./assets/providers/grok.svg";
import groqIcon from "./assets/providers/groq.svg";
import kiloIcon from "./assets/providers/kilocode.svg";
import kimiIcon from "./assets/providers/kimi.svg";
import kiroIcon from "./assets/providers/kiro.svg";
import minimaxIcon from "./assets/providers/minimax.svg";
import moonshotIcon from "./assets/providers/moonshot.svg";
import nousIcon from "./assets/providers/nous.svg";
import novitaIcon from "./assets/providers/novita.svg";
import openaiIcon from "./assets/providers/openai.svg";
import opencodeIcon from "./assets/providers/opencode.svg";
import openrouterIcon from "./assets/providers/openrouter.svg";
import windsurfIcon from "./assets/providers/windsurf.svg";
import zaiIcon from "./assets/providers/zai.svg";

export type VendorSlugId =
  | "anthropic" | "anthropic_api" | "openai" | "openai_admin" | "copilot"
  | "zai" | "openrouter" | "deepseek" | "kimi" | "kilo" | "novita"
  | "moonshot" | "grok" | "supergrok" | "antigravity" | "cursor" | "minimax"
  | "kiro" | "nous" | "opencode_go" | "commandcode" | "groq" | "windsurf";

export interface ProviderVisual {
  icon: string;
  accent: string;
  badge?: "API" | "SUPER";
}

function initials(name: string): string {
  const parts = name.trim().split(/\s+/).filter(Boolean);
  return (parts.length > 1 ? `${parts[0][0]}${parts[1][0]}` : parts[0]?.slice(0, 2) || "AI")
    .toUpperCase();
}

function monogramDataUri(name: string): string {
  const text = initials(name).replace(/[<>&"']/g, "");
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 48 48"><circle cx="24" cy="24" r="22" fill="#64748b"/><text x="24" y="29" text-anchor="middle" font-family="system-ui,sans-serif" font-size="15" font-weight="700" fill="white">${text}</text></svg>`;
  return `data:image/svg+xml,${encodeURIComponent(svg)}`;
}

export const DEFAULT_PROVIDER_VISUAL = (name: string): ProviderVisual => ({
  icon: monogramDataUri(name),
  accent: "var(--accent-neutral)",
});

export const PROVIDER_VISUAL: Record<VendorSlugId, ProviderVisual> = {
  anthropic: { icon: anthropicIcon, accent: "#d97757" },
  anthropic_api: { icon: anthropicIcon, accent: "#d97757", badge: "API" },
  openai: { icon: openaiIcon, accent: "#10a37f" },
  openai_admin: { icon: openaiIcon, accent: "#10a37f", badge: "API" },
  copilot: { icon: copilotIcon, accent: "#8b5cf6" },
  zai: { icon: zaiIcon, accent: "#38bdf8" },
  openrouter: { icon: openrouterIcon, accent: "#818cf8" },
  deepseek: { icon: deepseekIcon, accent: "#4d6bfe" },
  kimi: { icon: kimiIcon, accent: "#2563eb" },
  kilo: { icon: kiloIcon, accent: "#fbbf24" },
  novita: { icon: novitaIcon, accent: "#a855f7" },
  moonshot: { icon: moonshotIcon, accent: "#64748b" },
  grok: { icon: grokIcon, accent: "#94a3b8" },
  supergrok: { icon: grokIcon, accent: "#94a3b8", badge: "SUPER" },
  antigravity: { icon: antigravityIcon, accent: "#f43f5e" },
  cursor: { icon: cursorIcon, accent: "#22c55e" },
  minimax: { icon: minimaxIcon, accent: "#f97316" },
  kiro: { icon: kiroIcon, accent: "#22d3ee" },
  nous: { icon: nousIcon, accent: "#84cc16" },
  opencode_go: { icon: opencodeIcon, accent: "#eab308" },
  commandcode: { icon: monogramDataUri("Command Code"), accent: "#fb923c" },
  groq: { icon: groqIcon, accent: "#f59e0b" },
  windsurf: { icon: windsurfIcon, accent: "#06b6d4" },
};

export function providerVisual(id: string, name: string): ProviderVisual {
  return PROVIDER_VISUAL[id as VendorSlugId] ?? DEFAULT_PROVIDER_VISUAL(name);
}
