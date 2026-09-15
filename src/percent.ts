/** Contrato de porcentajes de resumen. Used + remaining siempre suman 100. */

export const ROUNDING_VECTORS: ReadonlyArray<[number, number, number]> = [
  [5.48, 5, 95],
  [5.5, 6, 94],
  [14.88, 15, 85],
  [38.2711, 38, 62],
  [99.6, 100, 0],
  [100, 100, 0],
  [-1, 0, 100],
];

export function summaryPercents(usedExact: number): { used: number; remaining: number } {
  const used = !Number.isFinite(usedExact) ? 0 : Math.round(Math.max(0, Math.min(100, usedExact)));
  return { used, remaining: 100 - used };
}

export type PercentageMode = "used" | "remaining";

export function normalizePercentageMode(value: string | null | undefined): PercentageMode {
  return value === "remaining" || value === "available" ? "remaining" : "used";
}
