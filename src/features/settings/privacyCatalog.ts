import type { PrivacyToggles, RetentionPolicy } from "@/types";

/**
 * What each tracked category says about itself (IMPLEMENTATION.md §22): what is
 * collected, why, where it is stored, for how long, and how to delete it.
 * Kept in step with docs/PRIVACY_REVIEW.md.
 */
export type SensorKey = "activeApplication" | "activeTime" | "idleDetection";

export type Disclosure = {
  key: SensorKey;
  label: string;
  collected: string;
  why: string;
  storedIn: string;
  /** Which retention setting governs it. */
  keptFor: keyof Pick<RetentionPolicy, "activityEventsDays" | "sessionsDays">;
  deleteWith: string;
};

const LOCAL = "On this device only, in the app's local database.";

export const SENSORS: Disclosure[] = [
  {
    key: "activeApplication",
    label: "Active application",
    collected:
      "The name of the app in front (for example Code) and when it changes. Never window titles, documents, or anything you type.",
    why: "To notice app switching and long stretches of work.",
    storedIn: LOCAL,
    keptFor: "activityEventsDays",
    deleteWith: "“Delete raw activity”, or “Delete all local data”.",
  },
  {
    key: "activeTime",
    label: "Active time",
    collected:
      "Nothing new. It groups the events above into sessions: when they started and ended, active minutes, app switches and a category.",
    why: "To show your day and notice long sessions.",
    storedIn: LOCAL,
    keptFor: "sessionsDays",
    deleteWith: "“Delete all local data”.",
  },
  {
    key: "idleDetection",
    label: "Idle detection",
    collected:
      "How many seconds since your last keyboard or mouse input, and how long each break from it lasted. Only the elapsed time, never which keys or where the pointer was.",
    why: "To tell time at the keyboard from time away.",
    storedIn: LOCAL,
    keptFor: "activityEventsDays",
    deleteWith: "“Delete raw activity”, or “Delete all local data”.",
  },
];

export type UnavailableKey = keyof Pick<
  PrivacyToggles,
  "windowTitle" | "keyboardMouseRhythm" | "calendar" | "cloudProcessing"
>;

/** Not built yet. They are shown so nothing is hidden, and they cannot be switched on. */
export const NOT_AVAILABLE: { key: UnavailableKey; label: string; note: string }[] = [
  { key: "windowTitle", label: "Window title", note: "Nothing is collected." },
  { key: "keyboardMouseRhythm", label: "Keyboard and mouse rhythm", note: "Nothing is collected." },
  { key: "calendar", label: "Calendar", note: "Nothing is read." },
  {
    key: "cloudProcessing",
    label: "Cloud processing",
    note: "Nothing leaves this device. There is no cloud processing.",
  },
];

export const days = (n: number) => `${n} ${n === 1 ? "day" : "days"}`;
