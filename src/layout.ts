export type RectLike = { left: number; right: number };

export function horizontalWheelDelta(deltaX: number, deltaY: number): number {
  return Math.abs(deltaX) >= Math.abs(deltaY) ? deltaX : deltaY;
}

export function activeItemScrollDelta(viewport: RectLike, item: RectLike): number {
  if (item.left < viewport.left) return item.left - viewport.left;
  if (item.right > viewport.right) return item.right - viewport.right;
  return 0;
}

export function cachePresentation(hasSnapshot: boolean, loading: boolean): "empty" | "cached" | "fresh" {
  if (!hasSnapshot) return "empty";
  return loading ? "cached" : "fresh";
}

export function elapsedLabel(updatedAt: string, now = Date.now()): { count: number; unit: "seconds" | "minutes" } | null {
  const timestamp = new Date(updatedAt).getTime();
  if (!Number.isFinite(timestamp)) return null;
  const seconds = Math.max(0, Math.floor((now - timestamp) / 1_000));
  return seconds < 60 ? { count: seconds, unit: "seconds" } : { count: Math.floor(seconds / 60), unit: "minutes" };
}
