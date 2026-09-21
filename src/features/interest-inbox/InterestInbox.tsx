import { useEffect, useRef, useState, type FormEvent } from "react";
import type { Interest } from "@/types";
import {
  useAddInterest,
  useArchiveInterest,
  useDeleteInterest,
  useInterests,
  useRestoreInterest,
} from "./useInterests";
import styles from "./InterestInbox.module.css";

export const MAX_LENGTH = 280;

export function InterestInbox() {
  const [showArchived, setShowArchived] = useState(false);

  return (
    <div className={styles.page}>
      <h1 className={styles.heading}>Interest Inbox</h1>
      <p className={styles.muted}>Things you&apos;re curious about, kept for later.</p>
      <AddForm />
      <List archived={false} />
      <label className={styles.toggle}>
        <input
          type="checkbox"
          checked={showArchived}
          onChange={(e) => setShowArchived(e.target.checked)}
        />{" "}
        Show archived
      </label>
      {showArchived && <List archived />}
    </div>
  );
}

/** One field, Enter to add, focus stays put: capturing a thought takes seconds. */
function AddForm() {
  const add = useAddInterest();
  const [text, setText] = useState("");
  const input = useRef<HTMLInputElement>(null);

  useEffect(() => {
    input.current?.focus();
  }, []);

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
      <label htmlFor="interest-text" className={styles.visuallyHidden}>
        Add an interest
      </label>
      <input
        id="interest-text"
        ref={input}
        value={text}
        maxLength={MAX_LENGTH}
        placeholder="Something you're curious about…"
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

function List({ archived }: { archived: boolean }) {
  const { data, isPending, isError } = useInterests(archived);
  const label = archived ? "Archived interests" : "Interests";

  if (isPending) return <p role="status">Loading…</p>;
  if (isError) return <p role="alert">Couldn&apos;t read your inbox from this device.</p>;
  if (data.length === 0) {
    return (
      <p className={styles.muted}>
        {archived ? "Nothing archived." : "Nothing here yet. Add whatever comes to mind."}
      </p>
    );
  }
  return (
    <ul className={styles.list} aria-label={label}>
      {data.map((interest) => (
        <Row key={interest.id} interest={interest} />
      ))}
    </ul>
  );
}

function Row({ interest }: { interest: Interest }) {
  const archive = useArchiveInterest();
  const restore = useRestoreInterest();
  const remove = useDeleteInterest();
  const [confirming, setConfirming] = useState(false);
  const archived = interest.archivedAt !== null;

  return (
    <li className={styles.row}>
      <div>
        <div>{interest.text}</div>
        <time dateTime={interest.createdAt} className={styles.muted}>
          Added {formatDay(interest.createdAt)}
        </time>
      </div>
      <div className={styles.actions}>
        {confirming ? (
          <>
            <span>Delete permanently?</span>
            <button type="button" onClick={() => remove.mutate(interest.id)}>
              Yes, delete
            </button>
            <button type="button" onClick={() => setConfirming(false)}>
              Cancel
            </button>
          </>
        ) : (
          <>
            {archived ? (
              <button
                type="button"
                aria-label={`Restore ${interest.text}`}
                onClick={() => restore.mutate(interest.id)}
              >
                Restore
              </button>
            ) : (
              <button
                type="button"
                aria-label={`Archive ${interest.text}`}
                onClick={() => archive.mutate(interest.id)}
              >
                Archive
              </button>
            )}
            <button
              type="button"
              aria-label={`Delete ${interest.text}`}
              onClick={() => setConfirming(true)}
            >
              Delete
            </button>
          </>
        )}
      </div>
    </li>
  );
}

function formatDay(iso: string): string {
  return new Date(iso).toLocaleDateString("en-US", { month: "short", day: "numeric" });
}
