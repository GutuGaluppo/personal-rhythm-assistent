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

export type SilentReason =
  | { kind: "silent_mode" }
  | { kind: "cooldown"; until: string }
  | { kind: "daily_ceiling"; limit: number }
  | { kind: "on_fire"; until: string };

export type PolicyDecision =
  | { kind: "silent"; reason: SilentReason }
  | { kind: "observe" }
  | { kind: "ask_checkin"; is_retry: boolean };

export type PolicyView = {
  silent: boolean;
  onFireUntil: string | null;
  cooldownUntil: string | null;
  shownToday: number;
  dailyLimit: number;
  retryPending: boolean;
  frequencyPromptDue: boolean;
};

export type PolicyDebug = { view: PolicyView; decision: PolicyDecision };

export type InterventionStep = "energy" | "positive" | "low";

export type InterventionView = {
  id: string;
  headline: string;
  question: string;
  /** Why this is being shown, as plain statements of the measurements. */
  reasons: string[];
  step: InterventionStep;
};

export type InterventionAnswer =
  | { kind: "energy"; value: "good" | "okay" | "low" }
  | {
      kind: "action";
      value: "continue" | "take_break" | "move" | "meditate" | "do_nothing" | "on_fire";
    }
  | { kind: "leave_me_alone" };

export type PauseKind = "silence" | "meditation" | "walking" | "stretching";
export type PausePhase = "setup" | "running" | "done";

export type PauseView = {
  phase: PausePhase;
  kind: PauseKind;
  startedAt: string | null;
  endsAt: string | null;
  durationSeconds: number | null;
};

export type Interest = {
  id: number;
  text: string;
  createdAt: string;
  archivedAt: string | null;
};
