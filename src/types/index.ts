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

export type SignalKind =
  "continuous_activity" | "insufficient_idle" | "rapid_switching" | "create_dominance";

export type Signal = {
  kind: SignalKind;
  active: boolean;
  evidence: {
    measured: number;
    threshold: number;
    unit: "minutes" | "ratio" | "per_hour";
    explanation: string;
  };
};

export type ContextAssessment = {
  assessedAt: string;
  signals: Signal[];
  activeSignalCount: number;
  decision: "observe" | "candidate_intervention";
};
