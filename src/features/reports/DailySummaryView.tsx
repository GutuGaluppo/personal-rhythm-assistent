import { useState, type FormEvent } from "react";
import { Card } from "@/components/ui/Card";
import { formatDuration, formatPercent } from "@/lib/utils/format";
import type { DailySummary } from "@/types";
import { useDailySummary, useSaveReflection } from "./useDailySummary";
import styles from "./DailySummaryView.module.css";

export const MAX_REFLECTION = 500;

export function DailySummaryView({ date, isToday }: { date: string; isToday: boolean }) {
  const { data, isPending, isError } = useDailySummary(date, isToday);

  if (isPending) return <p role="status">Loading…</p>;
  if (isError) return <p role="alert">Couldn&apos;t read this day from this device.</p>;
  return <Summary summary={data} />;
}

function Summary({ summary }: { summary: DailySummary }) {
  const empty = summary.activeMinutes === 0 && summary.pausesTaken === 0;

  return (
    <div className={styles.summary}>
      {empty ? (
        <p className={styles.muted}>Nothing recorded for this day.</p>
      ) : (
        <>
          <div className={styles.grid}>
            <Card title="The day" labelledBy="summary-day">
              <dl className={styles.stats}>
                <Stat label="Active time" value={formatDuration(summary.activeMinutes)} />
                <Stat
                  label="Longest session"
                  value={formatDuration(summary.longestSessionMinutes)}
                />
                <Stat label="Context switches" value={String(summary.contextSwitches)} />
                <Stat label="Pauses taken" value={String(summary.pausesTaken)} />
              </dl>
            </Card>

            <Card title="Where the time went" labelledBy="summary-categories">
              {summary.categoryDistribution.length === 0 ? (
                <p className={styles.muted}>No active time.</p>
              ) : (
                <ul className={styles.distribution}>
                  {summary.categoryDistribution.map((c) => (
                    <li key={c.category}>
                      <div className={styles.row}>
                        <span>{c.category}</span>
                        <span>
                          {formatDuration(c.minutes)} · {formatPercent(c.share)}
                        </span>
                      </div>
                      {/* Decoration only: the numbers beside it carry the meaning. */}
                      <div className={styles.track} aria-hidden="true">
                        <div className={styles.bar} style={{ width: formatPercent(c.share) }} />
                      </div>
                    </li>
                  ))}
                </ul>
              )}
            </Card>
          </div>
        </>
      )}
      {/* Keyed by day: showing another day starts its form afresh. */}
      <Reflection key={summary.date} summary={summary} />
    </div>
  );
}

function Stat({ label, value }: { label: string; value: string }) {
  return (
    <div className={styles.stat}>
      <dt>{label}</dt>
      <dd>{value}</dd>
    </div>
  );
}

/** Optional. Stays on this device. */
function Reflection({ summary }: { summary: DailySummary }) {
  const save = useSaveReflection(summary.date);
  const [text, setText] = useState(summary.reflection ?? "");

  const submit = (e: FormEvent) => {
    e.preventDefault();
    save.mutate(text);
  };

  return (
    <form onSubmit={submit} className={styles.reflection}>
      <label htmlFor="reflection">{summary.reflectiveQuestion}</label>
      <p className={styles.muted}>Optional. It stays on this device.</p>
      <textarea
        id="reflection"
        value={text}
        maxLength={MAX_REFLECTION}
        rows={3}
        onChange={(e) => {
          setText(e.target.value);
          if (save.isSuccess) save.reset();
        }}
      />
      <div className={styles.actions}>
        <button type="submit" disabled={save.isPending || text === (summary.reflection ?? "")}>
          Save
        </button>
        {save.isSuccess && <span role="status">Saved.</span>}
        {save.isError && <span role="alert">Couldn&apos;t save that.</span>}
      </div>
    </form>
  );
}
