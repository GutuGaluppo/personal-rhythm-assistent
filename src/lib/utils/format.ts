/** "45m", "1h 32m", "2h". Rounds to the nearest minute. */
export function formatDuration(minutes: number): string {
  const total = Math.max(0, Math.round(minutes));
  const h = Math.floor(total / 60);
  const m = total % 60;
  if (h === 0) return `${m}m`;
  return m === 0 ? `${h}h` : `${h}h ${m}m`;
}

/** "just now", "12m ago", "1h 5m ago". */
export function formatAgo(minutes: number): string {
  return Math.round(minutes) < 1 ? "just now" : `${formatDuration(minutes)} ago`;
}
