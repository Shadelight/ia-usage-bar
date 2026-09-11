export type UiLanguage = "es" | "en";

function calendarOrdinal(date: Date, timeZone?: string): number {
  const parts = new Intl.DateTimeFormat("en-US-u-ca-gregory", {
    timeZone,
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
  }).formatToParts(date);
  const pick = (type: Intl.DateTimeFormatPartTypes) =>
    Number(parts.find((part) => part.type === type)?.value ?? 0);
  return Date.UTC(pick("year"), pick("month") - 1, pick("day")) / 86_400_000;
}

export function formatResetRelative(
  resetAt: string | null | undefined,
  nowMs = Date.now(),
  language: UiLanguage = "es",
): string {
  if (!resetAt) return "";
  const end = new Date(resetAt).getTime();
  if (!Number.isFinite(end)) return "";
  const seconds = Math.max(0, Math.floor((end - nowMs) / 1000));
  if (seconds <= 0) return language === "es" ? "ahora" : "now";
  const days = Math.floor(seconds / 86_400);
  const hours = Math.floor((seconds % 86_400) / 3_600);
  const minutes = Math.floor((seconds % 3_600) / 60);
  if (days > 0) return `${days} d ${hours} h`;
  if (hours > 0) return `${hours} h ${minutes} min`;
  return `${minutes} min`;
}

export function formatResetAbsolute(
  resetAt: string | null | undefined,
  language: UiLanguage = "es",
  nowMs = Date.now(),
  timeZone?: string,
): string {
  if (!resetAt) return "";
  const date = new Date(resetAt);
  if (!Number.isFinite(date.getTime())) return "";
  const locale = language === "es" ? "es-ES" : "en-US";
  const dayDifference = calendarOrdinal(date, timeZone) - calendarOrdinal(new Date(nowMs), timeZone);
  const time = date.toLocaleTimeString(locale, {
    timeZone,
    hour: "2-digit",
    minute: "2-digit",
  });
  if (dayDifference === 0) return `${language === "es" ? "Hoy" : "Today"}, ${time}`;
  if (dayDifference === 1) return `${language === "es" ? "Mañana" : "Tomorrow"}, ${time}`;
  const shortDate = date
    .toLocaleDateString(locale, { timeZone, day: "numeric", month: "short" })
    .replace(/\./g, "");
  return `${shortDate}, ${time}`;
}
