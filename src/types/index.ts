export const CATEGORIES = [
  "Create",
  "Learn",
  "Explore",
  "Move",
  "Life",
  "People",
  "Recover",
  "Think",
  "Unknown",
] as const;
export type Category = (typeof CATEGORIES)[number];

export type ActivityState = "active" | "idle" | "unknown";

export type CurrentSession = {
  startedAt: string;
  elapsedMinutes: number;
  activeMinutes: number;
  category: Category;
  contextSwitches: number;
};

export type LastBreak = {
  minutes: number;
  /** null while the break is still going on. */
  endedMinutesAgo: number | null;
};

export type MyDay = {
  trackingEnabled: boolean;
  state: ActivityState;
  frontmostApplication: string | null;
  currentSession: CurrentSession | null;
  activeMinutesToday: number;
  contextSwitchesToday: number;
  lastBreak: LastBreak | null;
};

export type MappingSource = "user" | "default" | "unset";

export type AppMapping = {
  bundleId: string;
  applicationName: string;
  category: Category;
  source: MappingSource;
};
