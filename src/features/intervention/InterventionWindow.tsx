import { useEffect, useRef, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import {
  answerIntervention,
  dismissIntervention,
  getCurrentIntervention,
} from "@/lib/tauri/commands";
import type { InterventionAnswer, InterventionStep, InterventionView } from "@/types";
import styles from "./InterventionWindow.module.css";

type Choice = { label: string; answer: InterventionAnswer };

const CHOICES: Record<InterventionStep, Choice[]> = {
  energy: [
    { label: "Good", answer: { kind: "energy", value: "good" } },
    { label: "Okay", answer: { kind: "energy", value: "okay" } },
    { label: "Low", answer: { kind: "energy", value: "low" } },
  ],
  positive: [
    { label: "Continue", answer: { kind: "action", value: "continue" } },
    { label: "Take a break", answer: { kind: "action", value: "take_break" } },
  ],
  low: [
    { label: "Move", answer: { kind: "action", value: "move" } },
    { label: "Meditate", answer: { kind: "action", value: "meditate" } },
    { label: "Do nothing", answer: { kind: "action", value: "do_nothing" } },
  ],
};

/** The energy question comes from the backend (its wording rotates); later steps are fixed. */
const FOLLOW_UP: Record<Exclude<InterventionStep, "energy">, string> = {
  positive: "Would you like to keep going, or take a break?",
  low: "What sounds good right now?",
};

export function InterventionWindow() {
  const { data, isPending } = useQuery({
    queryKey: ["intervention"],
    queryFn: getCurrentIntervention,
    staleTime: Infinity,
  });
  const [override, setOverride] = useState<InterventionView | null | undefined>(undefined);

  if (isPending) return null;
  const view = override === undefined ? data : override;
  if (!view) return null;
  return <Check view={view} onNext={setOverride} />;
}

function Check({
  view,
  onNext,
}: {
  view: InterventionView;
  onNext: (next: InterventionView | null) => void;
}) {
  const firstAction = useRef<HTMLButtonElement>(null);

  // Keyboard users land on the first action of every step; Escape dismisses.
  useEffect(() => {
    firstAction.current?.focus();
  }, [view.step]);

  const dismiss = () => {
    void dismissIntervention(view.id).finally(() => onNext(null));
  };
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") dismiss();
    };
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  });

  const answer = (a: InterventionAnswer) => {
    void answerIntervention(view.id, a).then(onNext);
  };

  const question = view.step === "energy" ? view.question : FOLLOW_UP[view.step];

  return (
    <div
      className={styles.card}
      role="dialog"
      aria-modal="false"
      aria-labelledby="check-headline"
      aria-describedby="check-why"
    >
      <button type="button" className={styles.dismiss} aria-label="Dismiss" onClick={dismiss}>
        <span aria-hidden="true">×</span>
      </button>

      <p id="check-headline" className={styles.headline}>
        {view.headline}
      </p>
      <p className={styles.question}>{question}</p>

      <div role="group" aria-label="Your answer" className={styles.choices}>
        {CHOICES[view.step].map((choice, i) => (
          <button
            key={choice.label}
            type="button"
            ref={i === 0 ? firstAction : undefined}
            className={styles.choice}
            onClick={() => answer(choice.answer)}
          >
            {choice.label}
          </button>
        ))}
      </div>

      <div className={styles.secondary}>
        {view.step === "positive" && (
          <button
            type="button"
            className={styles.link}
            onClick={() => answer({ kind: "action", value: "on_fire" })}
          >
            I&apos;m on fire
          </button>
        )}
        <button
          type="button"
          className={styles.link}
          onClick={() => answer({ kind: "leave_me_alone" })}
        >
          Leave me alone for now
        </button>
      </div>

      <div id="check-why" className={styles.why}>
        <p className={styles.whyTitle}>Why you&apos;re seeing this</p>
        <ul>
          {view.reasons.map((reason) => (
            <li key={reason}>{reason}</li>
          ))}
        </ul>
      </div>
    </div>
  );
}
