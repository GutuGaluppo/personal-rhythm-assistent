import { useEffect, useRef, useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { PauseVisual } from "@/components/pause/PauseVisual";
import { endPause, getPauseView, startPause } from "@/lib/tauri/commands";
import { formatRemaining } from "@/lib/utils/format";
import { useReduceMotion } from "@/lib/utils/motion";
import { useNow } from "@/lib/utils/useNow";
import type { PauseKind, PauseView } from "@/types";
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

const PRESETS = [3, 5, 10] as const;
const CUSTOM = "custom";
type Choice = (typeof PRESETS)[number] | typeof CUSTOM;

const KEY = ["pause"] as const;

export function PauseWindow() {
  const client = useQueryClient();
  const { data: view, isPending } = useQuery({
    queryKey: KEY,
    queryFn: getPauseView,
    // The core owns the timer; this is only a safety net if an event is missed.
    refetchInterval: 5_000,
  });

  if (isPending || !view) return null;

  const begin = async (kind: PauseKind, minutes: number) => {
    client.setQueryData(KEY, await startPause(kind, minutes));
  };
  const finish = () => {
    void endPause().finally(() => client.setQueryData(KEY, null));
  };

  return view.phase === "setup" ? (
    <Setup suggested={view.kind} onStart={begin} onCancel={finish} />
  ) : (
    <Session view={view} onReturn={finish} />
  );
}

// ---- setup ---------------------------------------------------------------------------

function Setup({
  suggested,
  onStart,
  onCancel,
}: {
  suggested: PauseKind;
  onStart: (kind: PauseKind, minutes: number) => Promise<void>;
  onCancel: () => void;
}) {
  const reduceMotion = useReduceMotion();
  const [kind, setKind] = useState<PauseKind>(suggested);
  const [choice, setChoice] = useState<Choice>(5);
  const [custom, setCustom] = useState("15");
  const start = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    start.current?.focus();
  }, []);
  useEscape(onCancel);

  const minutes = choice === CUSTOM ? Number(custom) : choice;
  const valid = Number.isInteger(minutes) && minutes >= 1 && minutes <= 60;

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
      {!valid && (
        <p role="alert" className={styles.hint}>
          Choose between 1 and 60 minutes.
        </p>
      )}

      <div className={styles.actions}>
        <button
          ref={start}
          type="button"
          className={styles.primary}
          disabled={!valid}
          onClick={() => void onStart(kind, minutes)}
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

function Session({ view, onReturn }: { view: PauseView; onReturn: () => void }) {
  const reduceMotion = useReduceMotion();
  // Half a second is plenty for a display; correctness comes from the deadline.
  const now = useNow(500);
  const remaining = view.endsAt ? Date.parse(view.endsAt) - now : 0;
  const done = remaining <= 0;
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
      ) : (
        <>
          <p className={styles.prompt}>{PROMPTS[view.kind]}</p>
          <p role="timer" className={styles.remaining}>
            {formatRemaining(remaining)}
          </p>
          <div className={styles.actions}>
            <button ref={action} type="button" className={styles.quiet} onClick={onReturn}>
              Return now
            </button>
          </div>
        </>
      )}
    </main>
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
