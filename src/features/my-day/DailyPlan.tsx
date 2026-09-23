import { useRef, useState, type FormEvent } from "react";
import type { DailyPlanItem } from "@/types";
import {
  useAddDailyPlanItem,
  useDailyPlan,
  useDeleteDailyPlanItem,
  useToggleDailyPlanItem,
} from "./useDailyPlan";
import styles from "./DailyPlan.module.css";

export const MAX_LENGTH = 280;

export function DailyPlan() {
  return (
    <div className={styles.plan}>
      <AddForm />
      <List />
    </div>
  );
}

/** One field, Enter to add, focus stays put: jotting a task takes seconds. */
function AddForm() {
  const add = useAddDailyPlanItem();
  const [text, setText] = useState("");
  const input = useRef<HTMLInputElement>(null);

  const submit = (e: FormEvent) => {
    e.preventDefault();
    const value = text.trim();
    if (!value) return;
    add.mutate(value, {
      onSuccess: () => {
        setText("");
        input.current?.focus();
      },
    });
  };

  return (
    <form onSubmit={submit} className={styles.form}>
      <label htmlFor="daily-plan-text" className={styles.visuallyHidden}>
        Add something to today's plan
      </label>
      <input
        id="daily-plan-text"
        ref={input}
        value={text}
        maxLength={MAX_LENGTH}
        placeholder="What's on today?"
        onChange={(e) => setText(e.target.value)}
        className={styles.input}
      />
      <button type="submit" disabled={!text.trim() || add.isPending}>
        Add
      </button>
      {add.isError && <p role="alert">Couldn&apos;t save that. Try again.</p>}
    </form>
  );
}

function List() {
  const { data, isPending, isError } = useDailyPlan();

  if (isPending) return <p role="status">Loading…</p>;
  if (isError) return <p role="alert">Couldn&apos;t read today's plan from this device.</p>;
  if (data.length === 0) {
    return <p className={styles.muted}>Nothing planned yet. Add whatever you mean to get done.</p>;
  }
  return (
    <ul className={styles.list} aria-label="Today's plan">
      {data.map((item) => (
        <Row key={item.id} item={item} />
      ))}
    </ul>
  );
}

function Row({ item }: { item: DailyPlanItem }) {
  const toggle = useToggleDailyPlanItem();
  const remove = useDeleteDailyPlanItem();
  const done = item.doneAt !== null;

  return (
    <li className={styles.row}>
      <label className={styles.label}>
        <input
          type="checkbox"
          checked={done}
          onChange={() => toggle.mutate(item.id)}
          aria-label={item.text}
        />
        <span className={done ? styles.done : undefined}>{item.text}</span>
      </label>
      <button
        type="button"
        aria-label={`Delete ${item.text}`}
        onClick={() => remove.mutate(item.id)}
      >
        Delete
      </button>
    </li>
  );
}
