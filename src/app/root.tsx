import { InterventionWindow } from "@/features/intervention/InterventionWindow";
import { PauseWindow } from "@/features/pause/PauseWindow";
import { App } from "./App";

export type WindowKind = "main" | "intervention" | "pause";

/** One bundle serves every window; the small ones are opened with `?view=...`. */
export function windowKind(search: string): WindowKind {
  const view = new URLSearchParams(search).get("view");
  return view === "intervention" || view === "pause" ? view : "main";
}

export function isInterventionWindow(search: string): boolean {
  return windowKind(search) === "intervention";
}

export function Root() {
  switch (windowKind(window.location.search)) {
    case "intervention":
      return <InterventionWindow />;
    case "pause":
      return <PauseWindow />;
    default:
      return <App />;
  }
}
