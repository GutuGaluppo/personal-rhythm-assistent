import { useEffect, useRef, useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { PauseVisual } from "@/components/pause/PauseVisual";
import { endPause, getPauseView, setPauseReason, startPause } from "@/lib/tauri/commands";
import { formatDuration, formatRemaining } from "@/lib/utils/format";
import { useReduceMotion } from "@/lib/utils/motion";
import { useNow } from "@/lib/utils/useNow";
import type { PauseKind, PauseReason, PauseReasonKind, PauseView } from "@/types";
import styles from "./PauseWindow.module.css";

const KINDS: { kind: PauseKind; label: string }[] = [
  { kind: "silence", label: "Nothing" },
  { kind: "meditation", label: "Meditate" },
  { kind: "walking", label: "Walk" },
  { kind: "stretching", label: "Stretch" },
];

export const PROMPTS: Record<PauseKind, string> = {
  silence: "Nothing to do. Just pause.",
  meditation: "Close your eyes for a moment and follow your breathing.",
  walking: "Stand up and take a short walk.",
  stretching: "Stand up and stretch gently.",
};

const REASON_CHIPS: { kind: PauseReasonKind; label: string }[] = [
  { kind: "breakfast", label: "Breakfast" },
  { kind: "lunch", label: "Lunch" },
  { kind: "dinner", label: "Dinner" },
  { kind: "appointment", label: "Appointment" },
  { kind: "call", label: "Call" },
  { kind: "other", label: "Other" },
];

const PRESETS = [3, 5, 10] as const;
const CUSTOM = "custom";
type Choice = (typeof PRESETS)[number] | typeof CUSTOM;

const KEY = ["pause"] as const;

export function PauseWindow() {
  const client = useQueryClient();
  const {
    data: view,
    isPending,
    refetch,
  } = useQuery({
    queryKey: KEY,
    queryFn: getPauseView,
    // The core owns the timer; this is only a safety net if an event is missed.
    refetchInterval: 5_000,
  });

  // The window is hidden and reused between pauses rather than rebuilt (so
  // reopening is instant), so its page can be showing whatever it last showed.
  // Resync for real the moment it is shown or focused again, the same way
  // `useNow` catches a running countdown up to the true deadline.
  useEffect(() => {
    const resync = () => void refetch();
    document.addEventListener("visibilitychange", resync);
    window.addEventListener("focus", resync);
    return () => {
      document.removeEventListener("visibilitychange", resync);
      window.removeEventListener("focus", resync);
    };
  }, [refetch]);

  if (isPending || !view) return null;

  const begin = async (kind: PauseKind, minutes: number, reason: PauseReason | null) => {
    client.setQueryData(KEY, await startPause(kind, minutes, reason));
  };
  const finish = () => {
    void endPause().finally(() => client.setQueryData(KEY, null));
  };
  const setLocalReason = (reason: PauseReason | null) => {
    client.setQueryData(KEY, (prev: PauseView | undefined) => prev && { ...prev, reason });
  };
  const commitReason = async (reason: PauseReason | null) => {
    client.setQueryData(KEY, await setPauseReason(reason));
  };

  return view.phase === "setup" ? (
    <Setup suggested={view.kind} onStart={begin} onCancel={finish} />
  ) : (
    <Session
      view={view}
      onReturn={finish}
      onReasonChange={setLocalReason}
      onReasonCommit={commitReason}
    />
  );
}

// ---- setup ---------------------------------------------------------------------------

function Setup({
  suggested,
  onStart,
  onCancel,
}: {
  suggested: PauseKind;
  onStart: (kind: PauseKind, minutes: number, reason: PauseReason | null) => Promise<void>;
  onCancel: () => void;
}) {
  const reduceMotion = useReduceMotion();
  const [kind, setKind] = useState<PauseKind>(suggested);
  const [choice, setChoice] = useState<Choice>(5);
  const [custom, setCustom] = useState("15");
  const [reason, setReason] = useState<PauseReason | null>(null);
  const start = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    start.current?.focus();
  }, []);
  useEscape(onCancel);

  const minutes = choice === CUSTOM ? Number(custom) : choice;
  const minutesValid = Number.isInteger(minutes) && minutes >= 1 && minutes <= 60;
  const reasonValid = reason?.kind !== "other" || Boolean(reason.note?.trim());
  const valid = minutesValid && reasonValid;

  return (
    <main className={styles.page}>
      <PauseVisual settled={false} reduceMotion={reduceMotion} />
      <h1 className={styles.title}>Take a moment</h1>
      <p className={styles.prompt}>{PROMPTS[kind]}</p>

      <fieldset className={styles.group}>
        <legend>Kind of pause</legend>
        {KINDS.map((k) => (
          <label key={k.kind} className={styles.option}>
            <input
              type="radio"
              name="kind"
              checked={kind === k.kind}
              onChange={() => setKind(k.kind)}
            />
            {k.label}
          </label>
        ))}
      </fieldset>

      <fieldset className={styles.group}>
        <legend>How long</legend>
        {PRESETS.map((m) => (
          <label key={m} className={styles.option}>
            <input
              type="radio"
              name="length"
              checked={choice === m}
              onChange={() => setChoice(m)}
            />
            {m} min
          </label>
        ))}
        <label className={styles.option}>
          <input
            type="radio"
            name="length"
            checked={choice === CUSTOM}
            onChange={() => setChoice(CUSTOM)}
          />
          Custom
        </label>
        {choice === CUSTOM && (
          <label className={styles.custom}>
            Minutes
            <input
              type="number"
              min={1}
              max={60}
              value={custom}
              onChange={(e) => setCustom(e.target.value)}
              aria-invalid={!valid}
            />
          </label>
        )}
      </fieldset>

      <ReasonPicker value={reason} onChange={setReason} />

      {!minutesValid && (
        <p role="alert" className={styles.hint}>
          Choose between 1 and 60 minutes.
        </p>
      )}
      {minutesValid && !reasonValid && (
        <p role="alert" className={styles.hint}>
          Add a quick note for "Other," or pick a different reason.
        </p>
      )}

      <div className={styles.actions}>
        <button
          ref={start}
          type="button"
          className={styles.primary}
          disabled={!valid}
          onClick={() => void onStart(kind, minutes, reason)}
        >
          Start
        </button>
        <button type="button" className={styles.quiet} onClick={onCancel}>
          Not now
        </button>
      </div>
    </main>
  );
}

// ---- running and done -------------------------------------------------------------------

function Session({
  view,
  onReturn,
  onReasonChange,
  onReasonCommit,
}: {
  view: PauseView;
  onReturn: () => void;
  onReasonChange: (reason: PauseReason | null) => void;
  onReasonCommit: (reason: PauseReason | null) => Promise<void>;
}) {
  const reduceMotion = useReduceMotion();
  // Half a second is plenty for a display; correctness comes from the deadline.
  const now = useNow(500);
  // `endsAt` is null for a quick, untimed pause: there is no deadline to count
  // down to, so (unlike a timed pause) it is never done by the clock alone.
  const remaining = view.endsAt ? Date.parse(view.endsAt) - now : null;
  const done = remaining !== null && remaining <= 0;
  const elapsedMinutes = view.startedAt ? (now - Date.parse(view.startedAt)) / 60_000 : 0;
  const action = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    if (done) action.current?.focus();
  }, [done]);
  useEscape(onReturn);

  return (
    <main className={styles.page}>
      <PauseVisual settled reduceMotion={reduceMotion} />
      {done ? (
        <>
          <h1 className={styles.title}>Welcome back.</h1>
          <div className={styles.actions}>
            <button ref={action} type="button" className={styles.primary} onClick={onReturn}>
              Back to work
            </button>
          </div>
        </>
      ) : remaining !== null ? (
        <>
          <p className={styles.prompt}>{PROMPTS[view.kind]}</p>
          <p role="timer" className={styles.remaining}>
            {formatRemaining(remaining)}
          </p>
          <ReasonPicker value={view.reason} onChange={onReasonChange} onCommit={onReasonCommit} />
          <div className={styles.actions}>
            <button ref={action} type="button" className={styles.quiet} onClick={onReturn}>
              Return now
            </button>
          </div>
        </>
      ) : (
        <>
          <h1 className={styles.title}>You're on pause.</h1>
          <p className={styles.prompt}>{PROMPTS[view.kind]}</p>
          <p className={styles.remaining}>{formatDuration(elapsedMinutes)} so far</p>
          <ReasonPicker value={view.reason} onChange={onReasonChange} onCommit={onReasonCommit} />
          <div className={styles.actions}>
            <button ref={action} type="button" className={styles.primary} onClick={onReturn}>
              Back to work
            </button>
          </div>
        </>
      )}
    </main>
  );
}

// ---- reason ---------------------------------------------------------------------------

function ReasonPicker({
  value,
  onChange,
  onCommit,
}: {
  value: PauseReason | null;
  onChange: (reason: PauseReason | null) => void;
  /** A complete change: a chip pick, a clear, or a note losing focus/Enter.
   * Defaults to `onChange`, so a setup screen that only reads the value at
   * submit time doesn't need to care about the distinction. */
  onCommit?: (reason: PauseReason | null) => void;
}) {
  const commit = onCommit ?? onChange;

  const choose = (kind: PauseReasonKind) => {
    if (value?.kind === kind) {
      onChange(null);
      commit(null);
      return;
    }
    if (kind === "other") {
      onChange({ kind: "other", note: "" });
      return; // wait for a note before committing
    }
    const next: PauseReason = { kind, note: null };
    onChange(next);
    commit(next);
  };

  const commitNote = () => {
    const note = value?.kind === "other" ? value.note?.trim() : undefined;
    if (note) commit({ kind: "other", note });
  };

  return (
    <fieldset className={styles.group}>
      <legend>Why are you pausing? (optional)</legend>
      <div className={styles.chips}>
        {REASON_CHIPS.map((c) => (
          <button
            key={c.kind}
            type="button"
            className={value?.kind === c.kind ? styles.chipSelected : styles.chip}
            aria-pressed={value?.kind === c.kind}
            onClick={() => choose(c.kind)}
          >
            {c.label}
          </button>
        ))}
      </div>
      {value?.kind === "other" && (
        <label className={styles.custom}>
          What&apos;s up?
          <input
            type="text"
            value={value.note ?? ""}
            maxLength={140}
            onChange={(e) => onChange({ kind: "other", note: e.target.value })}
            onBlur={commitNote}
            onKeyDown={(e) => {
              if (e.key === "Enter") {
                e.preventDefault();
                commitNote();
              }
            }}
          />
        </label>
      )}
    </fieldset>
  );
}

function useEscape(handler: () => void) {
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") handler();
    };
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  }, [handler]);
}
