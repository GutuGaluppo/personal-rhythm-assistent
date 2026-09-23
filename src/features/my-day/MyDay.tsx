import { Card } from "@/components/ui/Card";
import { DailyPlan } from "@/features/my-day/DailyPlan";
import { InterestSuggestion } from "@/features/interest-inbox/InterestSuggestion";
import { openPause } from "@/lib/tauri/commands";
import { useNavigation } from "@/stores/navigation";
import { formatAgo, formatDuration } from "@/lib/utils/format";
import type { ActivityState, MyDay as MyDayData } from "@/types";
import { useMyDay } from "./useMyDay";
import styles from "./MyDay.module.css";

const STATE_LABEL: Record<ActivityState, string> = {
  active: "At the keyboard",
  idle: "Away",
  unknown: "Not measured",
};

export function MyDay() {
  const { data, isPending, isError } = useMyDay();

  return (
    <div className={styles.page}>
      <div className={styles.header}>
        <h1 className={styles.heading}>My Day</h1>
        <button type="button" onClick={() => void openPause()}>
          Take a break
        </button>
      </div>
      {isPending && <p role="status">Loading…</p>}
      {isError && <p role="alert">Couldn't read your day from this device.</p>}
      {data && <Content day={data} />}
    </div>
  );
}

function Content({ day }: { day: MyDayData }) {
  const go = useNavigation((s) => s.go);
  if (!day.trackingEnabled) {
    return (
      <p>
        Active time is switched off, so nothing is being tracked. You can turn it back on in
        Settings.
      </p>
    );
  }

  const session = day.currentSession;
  const onBreak = day.lastBreak !== null && day.lastBreak.endedMinutesAgo === null;

  return (
    <>
      <p className={styles.state}>
        {STATE_LABEL[day.state]}
        {day.frontmostApplication && <> · in {day.frontmostApplication}</>}
      </p>

      <div className={styles.grid}>
        <Card title="Current session" labelledBy="card-session">
          {session ? (
            <dl className={styles.stats}>
              <Stat label="Duration" value={formatDuration(session.elapsedMinutes)} />
              <Stat label="Category" value={session.category} />
              <Stat label="Context switches" value={String(session.contextSwitches)} />
            </dl>
          ) : (
            <p className={styles.muted}>
              {onBreak ? "You're on a break." : "No session right now."}
            </p>
          )}
        </Card>

        <Card title="Today" labelledBy="card-today">
          <dl className={styles.stats}>
            <Stat label="Active time" value={formatDuration(day.activeMinutesToday)} />
            <Stat label="Context switches" value={String(day.contextSwitchesToday)} />
          </dl>
          <p>
            <button type="button" onClick={() => go("history")}>
              See the day&apos;s summary
            </button>
          </p>
        </Card>

        <Card title="Last real break" labelledBy="card-break">
          <BreakText lastBreak={day.lastBreak} />
        </Card>

        <Card title="Today's plan" labelledBy="card-daily-plan" className={styles.wide}>
          <DailyPlan />
        </Card>

        <InterestSuggestion />
      </div>
    </>
  );
}

function BreakText({ lastBreak }: { lastBreak: MyDayData["lastBreak"] }) {
  if (!lastBreak) return <p className={styles.muted}>None recorded yet today.</p>;
  if (lastBreak.endedMinutesAgo === null) {
    return <p>In progress · {formatDuration(lastBreak.minutes)} so far</p>;
  }
  return (
    <p>
      Ended {formatAgo(lastBreak.endedMinutesAgo)} · lasted {formatDuration(lastBreak.minutes)}
    </p>
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
