/** Settings that only affect rendering: changing them must never restart the
 * CLI process (that would drop the live stream for a purely cosmetic
 * change). Everything else (cliPath, remotePollSeconds) needs a restart.
 * Kept in its own vscode-free module so the decision is unit-testable with a
 * plain stub instead of a real vscode.ConfigurationChangeEvent. */
export const RENDER_ONLY_KEYS = [
  "providers",
  "display",
  "showAllMetrics",
  "showResetInStatusBar",
  "percentageMode",
  "primaryMetric",
  "showProviderIcons",
  "showStaleIndicator",
] as const;

export function isRenderOnlyChange(affectsConfiguration: (section: string) => boolean): boolean {
  if (!affectsConfiguration("iaUsage")) return false;
  if (affectsConfiguration("iaUsage.cliPath") || affectsConfiguration("iaUsage.remotePollSeconds")) return false;
  return RENDER_ONLY_KEYS.some((key) => affectsConfiguration(`iaUsage.${key}`));
}
