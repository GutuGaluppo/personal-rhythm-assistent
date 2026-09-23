import { useState, type KeyboardEvent } from "react";
import { Button } from "@/components/ui/Button";
import { DailySummaryView } from "@/features/reports/DailySummaryView";
import { useSummaryDays } from "@/features/reports/useDailySummary";
import { WeeklyReviewView } from "@/features/reports/WeeklyReviewView";
import { formatDate } from "@/lib/utils/format";
import styles from "./History.module.css";

const TABS = [
  { id: "day", label: "Day" },
  { id: "week", label: "Week" },
] as const;
type TabId = (typeof TABS)[number]["id"];

export function History() {
  const [tab, setTab] = useState<TabId>("day");

  // Left/Right move between the tabs, as a tab list should.
  const onKeyDown = (e: KeyboardEvent) => {
    if (e.key !== "ArrowRight" && e.key !== "ArrowLeft") return;
    const at = TABS.findIndex((t) => t.id === tab);
    const next = TABS[(at + (e.key === "ArrowRight" ? 1 : TABS.length - 1)) % TABS.length];
    setTab(next.id);
    document.getElementById(`tab-${next.id}`)?.focus();
  };

  return (
    <div className={styles.page}>
      <h1 className={styles.heading}>History</h1>

      <div role="tablist" aria-label="Period" className={styles.tabs} onKeyDown={onKeyDown}>
        {TABS.map((t) => (
          <button
            key={t.id}
            id={`tab-${t.id}`}
            type="button"
            role="tab"
            aria-selected={tab === t.id}
            aria-controls={`panel-${t.id}`}
            tabIndex={tab === t.id ? 0 : -1}
            className={styles.tab}
            onClick={() => setTab(t.id)}
          >
            {t.label}
          </button>
        ))}
      </div>

      <div role="tabpanel" id={`panel-${tab}`} aria-labelledby={`tab-${tab}`}>
        {tab === "day" ? <DayPanel /> : <WeeklyReviewView />}
      </div>
    </div>
  );
}

function DayPanel() {
  const { data: days, isPending, isError } = useSummaryDays();
  // Index into `days`, which is newest first: 0 is today.
  const [index, setIndex] = useState(0);

  if (isPending) return <p role="status">Loading…</p>;
  if (isError) return <p role="alert">Couldn&apos;t read your history from this device.</p>;

  const date = days[Math.min(index, days.length - 1)];
  const isToday = index === 0;

  return (
    <>
      <div className={styles.nav}>
        <Button
          variant="ghost"
          onClick={() => setIndex(index + 1)}
          disabled={index >= days.length - 1}
        >
          Earlier day
        </Button>
        <h2 className={styles.day} aria-live="polite">
          {formatDate(date)}
          <span className={styles.muted}>{isToday ? " · today so far" : ""}</span>
        </h2>
        <Button variant="ghost" onClick={() => setIndex(index - 1)} disabled={index === 0}>
          Later day
        </Button>
      </div>

      <DailySummaryView date={date} isToday={isToday} />
    </>
  );
}
