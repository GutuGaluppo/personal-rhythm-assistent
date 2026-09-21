import { useState } from "react";
import { DailySummaryView } from "@/features/reports/DailySummaryView";
import { useSummaryDays } from "@/features/reports/useDailySummary";
import { formatDate } from "@/lib/utils/format";
import styles from "./History.module.css";

export function History() {
  const { data: days, isPending, isError } = useSummaryDays();
  // Index into `days`, which is newest first: 0 is today.
  const [index, setIndex] = useState(0);

  if (isPending) return <p role="status">Loading…</p>;
  if (isError) return <p role="alert">Couldn&apos;t read your history from this device.</p>;

  const date = days[Math.min(index, days.length - 1)];
  const isToday = index === 0;

  return (
    <div className={styles.page}>
      <h1 className={styles.heading}>History</h1>

      <div className={styles.nav}>
        <button
          type="button"
          onClick={() => setIndex(index + 1)}
          disabled={index >= days.length - 1}
        >
          Earlier day
        </button>
        <h2 className={styles.day} aria-live="polite">
          {formatDate(date)}
          <span className={styles.muted}>{isToday ? " · today so far" : ""}</span>
        </h2>
        <button type="button" onClick={() => setIndex(index - 1)} disabled={index === 0}>
          Later day
        </button>
      </div>

      <DailySummaryView date={date} isToday={isToday} />
    </div>
  );
}
