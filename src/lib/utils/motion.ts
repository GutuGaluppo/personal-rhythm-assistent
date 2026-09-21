import { useEffect, useState } from "react";

/**
 * Reduce Motion is honoured from the first release (IMPLEMENTATION.md §2 rule 17):
 * the system setting always applies, and the app adds its own switch.
 * The preference lives in this webview's storage, which every app window shares.
 */
export type MotionPreference = "system" | "reduce";

const KEY = "reduceMotion";
const CHANGE_EVENT = "motion-preference-change";
const SYSTEM_QUERY = "(prefers-reduced-motion: reduce)";

export function readMotionPreference(): MotionPreference {
  try {
    return window.localStorage.getItem(KEY) === "reduce" ? "reduce" : "system";
  } catch {
    return "system"; // storage can be unavailable; never let that break the UI
  }
}

export function writeMotionPreference(preference: MotionPreference): void {
  try {
    window.localStorage.setItem(KEY, preference);
  } catch {
    /* keep the in-memory effect below even if it cannot be saved */
  }
  applyMotionPreference();
  window.dispatchEvent(new Event(CHANGE_EVENT));
}

export function systemPrefersReducedMotion(): boolean {
  return typeof window.matchMedia === "function" && window.matchMedia(SYSTEM_QUERY).matches;
}

export function shouldReduceMotion(preference: MotionPreference): boolean {
  return preference === "reduce" || systemPrefersReducedMotion();
}

/** Lets plain CSS follow the same decision (see `[data-reduce-motion]` in tokens.css). */
export function applyMotionPreference(): void {
  document.documentElement.dataset.reduceMotion = String(
    shouldReduceMotion(readMotionPreference()),
  );
}

export function useReduceMotion(): boolean {
  const [reduce, setReduce] = useState(() => shouldReduceMotion(readMotionPreference()));

  useEffect(() => {
    const update = () => setReduce(shouldReduceMotion(readMotionPreference()));
    const media = typeof window.matchMedia === "function" ? window.matchMedia(SYSTEM_QUERY) : null;
    media?.addEventListener?.("change", update);
    window.addEventListener(CHANGE_EVENT, update);
    window.addEventListener("storage", update);
    return () => {
      media?.removeEventListener?.("change", update);
      window.removeEventListener(CHANGE_EVENT, update);
      window.removeEventListener("storage", update);
    };
  }, []);

  return reduce;
}
