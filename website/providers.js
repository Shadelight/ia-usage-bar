/** Landing provider chips. Keep ids/names aligned with VendorId in iausage-core. */

export const PROVIDERS = [
  { id: "anthropic", name: "Claude Code", icon: "anthropic.svg" },
  { id: "openai", name: "Codex / ChatGPT", icon: "openai.svg" },
  { id: "cursor", name: "Cursor", icon: "cursor.svg" },
  { id: "antigravity", name: "Antigravity", icon: "antigravity.svg" },
  { id: "copilot", name: "GitHub Copilot", icon: "copilot.svg" },
  { id: "openai_admin", name: "OpenAI API · Admin", icon: "openai.svg" },
  { id: "anthropic_api", name: "Anthropic API", icon: "anthropic.svg" },
  { id: "openrouter", name: "OpenRouter", icon: "openrouter.svg" },
  { id: "zai", name: "Z.AI / GLM", icon: "zai.svg" },
  { id: "deepseek", name: "DeepSeek", icon: "deepseek.svg" },
  { id: "grok", name: "Grok (xAI)", icon: "grok.svg" },
  { id: "supergrok", name: "SuperGrok", icon: "grok.svg" },
  { id: "groq", name: "Groq", icon: "groq.svg" },
  { id: "kiro", name: "Kiro", icon: "kiro.svg" },
  { id: "windsurf", name: "Windsurf", icon: "windsurf.svg" },
  { id: "opencode_go", name: "OpenCode Go", icon: "opencode.svg" },
  { id: "kimi", name: "Kimi", icon: "kimi.svg" },
  { id: "kilo", name: "Kilo", icon: "kilocode.svg" },
  { id: "novita", name: "Novita", icon: "novita.svg" },
  { id: "moonshot", name: "Moonshot", icon: "moonshot.svg" },
  { id: "minimax", name: "MiniMax", icon: "minimax.svg" },
  { id: "nous", name: "Nous Research", icon: "nous.svg" },
  { id: "commandcode", name: "Command Code", icon: null },
];

export function iconSrc(provider) {
  return provider.icon ? `./assets/providers/${provider.icon}` : "";
}

/** Same monogram fallback the desktop app uses when no official SVG exists. */
export function monogramDataUri(name) {
  const parts = String(name).trim().split(/\s+/).filter(Boolean);
  const text = (parts.length > 1 ? `${parts[0][0]}${parts[1][0]}` : parts[0]?.slice(0, 2) || "AI")
    .toUpperCase()
    .replace(/[<>&"']/g, "");
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 48 48"><circle cx="24" cy="24" r="22" fill="#64748b"/><text x="24" y="29" text-anchor="middle" font-family="system-ui,sans-serif" font-size="15" font-weight="700" fill="white">${text}</text></svg>`;
  return `data:image/svg+xml,${encodeURIComponent(svg)}`;
}
