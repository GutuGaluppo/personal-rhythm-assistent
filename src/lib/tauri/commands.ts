import { invoke } from "@tauri-apps/api/core";
import type {
  AppMapping,
  Category,
  ActivityEvent,
  ContextAssessment,
  DailySummary,
  DataOverview,
  InterventionAnswer,
  InterventionView,
  Interest,
  MyDay,
  PauseKind,
  PauseView,
  PolicyDebug,
  PrivacyToggles,
  RetentionPolicy,
  RetentionReport,
  WeeklyReview,
} from "@/types";

/** Typed wrappers around the Rust commands. Nothing else calls `invoke` directly. */

export const getMyDay = () => invoke<MyDay>("get_my_day");

export const listAppMappings = () => invoke<AppMapping[]>("list_app_mappings");

export const setAppCategory = (bundleId: string, category: Category) =>
  invoke<void>("set_app_category", { bundleId, category });

export const resetAppCategory = (bundleId: string) =>
  invoke<void>("reset_app_category", { bundleId });

export const getContextAssessment = () => invoke<ContextAssessment>("get_context_assessment");

export const getPolicyDebug = () => invoke<PolicyDebug>("get_policy_debug");

// The check-in window may call only these three (see capabilities/intervention.json).
export const getCurrentIntervention = () =>
  invoke<InterventionView | null>("get_current_intervention");

/** Resolves to the next screen, or null once the check-in is over. */
export const answerIntervention = (id: string, answer: InterventionAnswer) =>
  invoke<InterventionView | null>("answer_intervention", { id, answer });

export const dismissIntervention = (id: string) => invoke<void>("dismiss_intervention", { id });

/** Developer preview; records nothing. */
export const debugShowIntervention = () => invoke<void>("debug_show_intervention");

// The pause window may call only these three (see capabilities/pause.json).
export const getPauseView = () => invoke<PauseView | null>("get_pause_view");

/** `minutes` is 1-60; the presets are 3, 5 and 10. */
export const startPause = (kind: PauseKind, minutes: number) =>
  invoke<PauseView>("start_pause", { kind, minutes });

/** Coming back, or ending early: the core clears the pause and closes the window. */
export const endPause = () => invoke<void>("end_pause");

/** From the main window: opens the pause window on its setup screen. */
export const openPause = () => invoke<void>("open_pause");

// ---- Interest Inbox ----

export const listInterests = (archived: boolean) =>
  invoke<Interest[]>("list_interests", { archived });

export const addInterest = (text: string) => invoke<Interest>("add_interest", { text });

export const archiveInterest = (id: number) => invoke<boolean>("archive_interest", { id });

export const restoreInterest = (id: number) => invoke<boolean>("restore_interest", { id });

/** Permanent. */
export const deleteInterest = (id: number) => invoke<boolean>("delete_interest", { id });

/** One suggestion per day, the same one all day; null when the inbox has nothing active. */
export const getInterestSuggestion = () => invoke<Interest | null>("get_interest_suggestion");

// ---- Daily summary ----

/** `date` is YYYY-MM-DD in local time; omitted means today. */
export const getDailySummary = (date?: string) =>
  invoke<DailySummary>("get_daily_summary", { date: date ?? null });

/** Days that have something to show, newest first. Always includes today. */
export const listSummaryDays = () => invoke<string[]>("list_summary_days");

/** The user's own words about a day. Empty text clears it. */
export const saveReflection = (date: string, text: string) =>
  invoke<void>("save_reflection", { date, text });

// ---- Weekly review ----

/** The last seven local days, ending today. */
export const getWeeklyReview = () => invoke<WeeklyReview>("get_weekly_review");

// ---- Privacy ----

export const getPrivacyToggles = () => invoke<PrivacyToggles>("get_privacy_toggles");

/** Resolves to what was actually stored: features that are not built yet stay off. */
export const setPrivacyToggles = (toggles: PrivacyToggles) =>
  invoke<PrivacyToggles>("set_privacy_toggles", { toggles });

export const getRetentionPolicy = () => invoke<RetentionPolicy>("get_retention_policy");

/** Applies straight away and resolves to what was removed. */
export const setRetentionPolicy = (policy: RetentionPolicy) =>
  invoke<RetentionReport>("set_retention_policy", { policy });

export const getDataOverview = () => invoke<DataOverview>("get_data_overview");

/** The newest raw events, newest first. At most 200. */
export const listRecentActivityEvents = (limit: number) =>
  invoke<ActivityEvent[]>("list_recent_activity_events", { limit });

/** Removes the raw app and idle events only. Resolves to how many. */
export const deleteRawData = () => invoke<number>("delete_raw_data");

/** Removes everything the app stored on this device. */
export const deleteAllLocalData = () => invoke<void>("delete_all_local_data");
