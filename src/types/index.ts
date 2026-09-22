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

export type CategoryShare = {
  category: Category;
  minutes: number;
  /** 0 to 1 of the day's active time. */
  share: number;
};

export type DailySummary = {
  /** Local calendar day, YYYY-MM-DD. */
  date: string;
  isFinal: boolean;
  activeMinutes: number;
  categoryDistribution: CategoryShare[];
  longestSessionMinutes: number;
  contextSwitches: number;
  pausesTaken: number;
  reflectiveQuestion: string;
  reflection: string | null;
};

export type WeekDay = {
  date: string;
  activeMinutes: number;
  longestSessionMinutes: number;
  contextSwitches: number;
  sessions: number;
  averageSessionMinutes: number | null;
};

export type Observation = {
  /** Stable key for the rule that produced it. */
  id: string;
  text: string;
  /** The stored numbers the text was made from. */
  evidence: string;
};

export type WeeklyReview = {
  from: string;
  to: string;
  activeMinutes: number;
  days: WeekDay[];
  categoryDistribution: CategoryShare[];
  sessions: {
    count: number;
    averageMinutes: number;
    longestMinutes: number;
    longSessionThresholdMinutes: number;
    longSessions: number;
    longSessionDays: number;
  };
  switching: {
    total: number;
    perActiveHour: number;
    busiestDay: { date: string; perActiveHour: number } | null;
  };
  checkIns: {
    shown: number;
    accepted: number;
    declined: number;
    ignored: number;
    onFire: number;
  };
  observations: Observation[];
  reflectiveQuestion: string;
};

export type PrivacyToggles = {
  activeApplication: boolean;
  activeTime: boolean;
  idleDetection: boolean;
  // Not built yet: always off, and cannot be switched on.
  windowTitle: boolean;
  keyboardMouseRhythm: boolean;
  calendar: boolean;
  cloudProcessing: boolean;
};

export type RetentionPolicy = {
  activityEventsDays: number;
  sessionsDays: number;
  /** null keeps daily summaries indefinitely. */
  dailySummariesDays: number | null;
};

export type RetentionReport = {
  activityEventsDeleted: number;
  sessionsDeleted: number;
  interventionsDeleted: number;
  pausesDeleted: number;
  dailySummariesDeleted: number;
};

export type TableStats = { count: number; oldest: string | null; newest: string | null };

export type DataOverview = {
  activityEvents: TableStats;
  sessions: TableStats;
  checkIns: TableStats;
  pauses: TableStats;
  dailySummaries: TableStats;
  interests: TableStats;
  appCategoryOverrides: number;
  databaseBytes: number;
};

/** A raw event exactly as stored: app names, switches and idle lengths, nothing else. */
export type ActivityEvent =
  | { type: "active_application"; timestamp: string; bundleId: string; applicationName: string }
  | { type: "application_switch"; timestamp: string; fromBundleId: string; toBundleId: string }
  | { type: "idle"; timestamp: string; seconds: number };
