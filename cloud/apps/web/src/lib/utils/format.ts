/** Format a percentage (0–100) for display, or an em dash when unknown. */
export function formatPct(value: number | null): string {
  if (value === null || Number.isNaN(value)) return '—';
  return `${Math.round(value)}%`;
}

/** Compact uptime from seconds, e.g. 1043000 → "12d 1h". */
export function formatUptime(seconds: number | null): string {
  if (seconds === null || seconds <= 0) return '—';
  const d = Math.floor(seconds / 86400);
  const h = Math.floor((seconds % 86400) / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  if (d > 0) return `${d}d ${h}h`;
  if (h > 0) return `${h}h ${m}m`;
  return `${m}m`;
}
