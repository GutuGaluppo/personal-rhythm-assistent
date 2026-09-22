import { useState, type FormEvent } from "react";
import type { RetentionPolicy, RetentionReport } from "@/types";
import { useRetentionPolicy, useSetRetention } from "./usePrivacy";
import styles from "./Settings.module.css";

function removedCount(r: RetentionReport): number {
  return (
    r.activityEventsDeleted +
    r.sessionsDeleted +
    r.interventionsDeleted +
    r.pausesDeleted +
    r.dailySummariesDeleted
  );
}

export function RetentionSettings() {
  const { data, isPending, isError } = useRetentionPolicy();
  return (
    <section aria-labelledby="retention-heading" className={styles.block}>
      <h2 id="retention-heading">How long data is kept</h2>
      {isPending && <p role="status">Loading…</p>}
      {isError && <p role="alert">Couldn&apos;t read the retention settings.</p>}
      {data && <Form policy={data} />}
    </section>
  );
}

function Form({ policy }: { policy: RetentionPolicy }) {
  const save = useSetRetention();
  const [events, setEvents] = useState(String(policy.activityEventsDays));
  const [sessions, setSessions] = useState(String(policy.sessionsDays));
  const [forever, setForever] = useState(policy.dailySummariesDays === null);
  const [summaries, setSummaries] = useState(String(policy.dailySummariesDays ?? 365));

  const whole = (v: string) => (/^\d+$/.test(v) && Number(v) >= 1 ? Number(v) : null);
  const next: RetentionPolicy | null =
    whole(events) && whole(sessions) && (forever || whole(summaries))
      ? {
          activityEventsDays: Number(events),
          sessionsDays: Number(sessions),
          dailySummariesDays: forever ? null : Number(summaries),
        }
      : null;
  const changed =
    next !== null &&
    (next.activityEventsDays !== policy.activityEventsDays ||
      next.sessionsDays !== policy.sessionsDays ||
      next.dailySummariesDays !== policy.dailySummariesDays);

  const submit = (e: FormEvent) => {
    e.preventDefault();
    if (next) save.mutate(next);
  };

  return (
    <form onSubmit={submit} className={styles.form}>
      <p className={styles.muted}>
        A change applies right away: anything older than the new limit is removed.
      </p>
      <label>
        Raw activity (days)
        <input type="number" min={1} value={events} onChange={(e) => setEvents(e.target.value)} />
      </label>
      <label>
        Sessions and check-ins (days)
        <input
          type="number"
          min={1}
          value={sessions}
          onChange={(e) => setSessions(e.target.value)}
        />
      </label>

      <fieldset className={styles.fieldset}>
        <legend>Daily summaries</legend>
        <label>
          <input
            type="radio"
            name="summaries"
            checked={forever}
            onChange={() => setForever(true)}
          />{" "}
          Keep indefinitely
        </label>
        <label>
          <input
            type="radio"
            name="summaries"
            checked={!forever}
            onChange={() => setForever(false)}
          />{" "}
          Keep for
          <input
            type="number"
            min={1}
            value={summaries}
            disabled={forever}
            aria-label="Days to keep daily summaries"
            onChange={(e) => setSummaries(e.target.value)}
          />{" "}
          days
        </label>
      </fieldset>

      {!next && <p role="alert">Each period must be a whole number of days, at least 1.</p>}

      <div className={styles.actions}>
        <button type="submit" disabled={!changed || save.isPending}>
          Save
        </button>
        {save.isSuccess && (
          <span role="status">
            {removedCount(save.data) === 0
              ? "Saved. Nothing needed removing."
              : `Saved. Removed ${removedCount(save.data)} older ${removedCount(save.data) === 1 ? "record" : "records"}.`}
          </span>
        )}
        {save.isError && <span role="alert">Couldn&apos;t save that.</span>}
      </div>
    </form>
  );
}
