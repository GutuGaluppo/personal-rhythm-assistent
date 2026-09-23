import { useEffect, useState } from "react";

/**
 * The current time, refreshed on an interval and again the moment the window is
 * shown or focused. Browsers throttle timers in windows that are hidden, so a
 * display must always derive what it shows from a deadline and *this* clock,
 * never by counting ticks.
 */
export function useNow(intervalMs: number): number {
  const [now, setNow] = useState(() => Date.now());

  useEffect(() => {
    const refresh = () => setNow(Date.now());
    const id = window.setInterval(refresh, intervalMs);
    document.addEventListener("visibilitychange", refresh);
    window.addEventListener("focus", refresh);
    return () => {
      window.clearInterval(id);
      document.removeEventListener("visibilitychange", refresh);
      window.removeEventListener("focus", refresh);
    };
  }, [intervalMs]);

  return now;
}
