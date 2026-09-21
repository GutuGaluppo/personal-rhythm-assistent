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

/** Calm, coarse remaining time for a pause: no ticking seconds. */
export function formatRemaining(ms: number): string {
  if (ms <= 0) return "Time's up";
  if (ms <= 60_000) return "Less than a minute left";
  const minutes = Math.ceil(ms / 60_000);
  return `About ${minutes} min left`;
}

export function formatPercent(share: number): string {
  return `${Math.round(share * 100)}%`;
}

/** "Wednesday, March 4" from a local YYYY-MM-DD, without time zone surprises. */
export function formatDate(isoDate: string): string {
  const [y, m, d] = isoDate.split("-").map(Number);
  return new Date(y, m - 1, d).toLocaleDateString("en-US", {
    weekday: "long",
    month: "long",
    day: "numeric",
  });
}
