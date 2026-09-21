import { InterventionWindow } from "@/features/intervention/InterventionWindow";
import { App } from "./App";

/** One bundle serves both windows; the check-in window is opened with `?view=intervention`. */
export function isInterventionWindow(search: string): boolean {
  return new URLSearchParams(search).get("view") === "intervention";
}

export function Root() {
  return isInterventionWindow(window.location.search) ? <InterventionWindow /> : <App />;
}
