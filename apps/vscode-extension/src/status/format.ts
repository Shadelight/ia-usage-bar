export function formatDuration(seconds: number): string {
  if (seconds < 3_600) return `${Math.max(1, Math.floor(seconds / 60))} min`;
  if (seconds < 86_400) {
    const hours = Math.floor(seconds / 3_600);
    const minutes = Math.floor((seconds % 3_600) / 60);
    return minutes > 0 ? `${hours} h ${minutes} min` : `${hours} h`;
  }
  const days = Math.floor(seconds / 86_400);
  const hours = Math.floor((seconds % 86_400) / 3_600);
  return hours > 0 ? `${days} d ${hours} h` : `${days} d`;
}

export function formatAge(updatedAt: string): string {
  const seconds = Math.max(0, (Date.now() - new Date(updatedAt).getTime()) / 1000);
  if (seconds < 60) return "un momento";
  return formatDuration(seconds);
}

/** Neutralizes Markdown syntax in CLI-supplied text. isTrusted stays off (no
 * command: links), but escaping still stops a crafted provider/quota name
 * from breaking out of the bold/image markup it's interpolated into. */
export function escapeMd(text: string): string {
  return text.replace(/[\\`*_{}[\]()#+\-.!<>|]/g, (ch) => `\\${ch}`);
}
