import { useQuery } from "@tanstack/react-query";
import { CategoryBars } from "@/components/rhythm/CategoryBars";
import { Card } from "@/components/ui/Card";
import { getWeeklyReview } from "@/lib/tauri/commands";
import { formatDuration, formatShortDate, formatWeekday } from "@/lib/utils/format";
import type { WeeklyReview } from "@/types";
import styles from "./WeeklyReviewView.module.css";

export function WeeklyReviewView() {
  const { data, isPending, isError } = useQuery({
    queryKey: ["weekly-review"],
    queryFn: getWeeklyReview,
    refetchInterval: 60_000,
  });

  if (isPending) return <p role="status">Loading…</p>;
  if (isError) return <p role="alert">Couldn&apos;t read your week from this device.</p>;
  return <Review review={data} />;
}

function Review({ review }: { review: WeeklyReview }) {
  const { sessions, switching, checkIns } = review;
  const quiet = review.activeMinutes === 0;

  return (
    <div className={styles.review}>
      <p className={styles.muted}>
        The last 7 days · {formatShortDate(review.from)} – {formatShortDate(review.to)}
      </p>

      <section aria-labelledby="observations-heading">
        <h2 id="observations-heading">What stands out</h2>
        <ul className={styles.observations} aria-labelledby="observations-heading">
          {review.observations.map((o) => (
            <li key={o.id}>
              <span>{o.text}</span>
              <span className={styles.evidence}>{o.evidence}</span>
            </li>
          ))}
        </ul>
      </section>

      {quiet ? (
        <p className={styles.muted}>Nothing recorded this week.</p>
      ) : (
        <div className={styles.grid}>
          <Card title="Where the time went" labelledBy="week-categories">
            <CategoryBars shares={review.categoryDistribution} />
          </Card>

          <Card title="Sessions" labelledBy="week-sessions">
            <dl className={styles.stats}>
              <Stat label="Sessions" value={String(sessions.count)} />
              <Stat label="Average length" value={formatDuration(sessions.averageMinutes)} />
              <Stat label="Longest" value={formatDuration(sessions.longestMinutes)} />
              <Stat
                label={`Sessions of ${formatDuration(sessions.longSessionThresholdMinutes)} or more`}
                value={String(sessions.longSessions)}
              />
            </dl>
          </Card>

          <Card title="App switching" labelledBy="week-switching">
            <dl className={styles.stats}>
              <Stat label="Switches" value={String(switching.total)} />
              <Stat label="Per active hour" value={String(Math.round(switching.perActiveHour))} />
              {switching.busiestDay && (
                <Stat
                  label="Busiest day"
                  value={`${formatWeekday(switching.busiestDay.date)} · ${Math.round(switching.busiestDay.perActiveHour)} per hour`}
                />
              )}
            </dl>
          </Card>
        </div>
      )}

      <Card title="Check-ins" labelledBy="week-checkins">
        <dl className={styles.stats}>
          <Stat label="Shown" value={String(checkIns.shown)} />
          <Stat label="Took a break" value={String(checkIns.accepted)} />
          <Stat label="Chose to continue" value={String(checkIns.declined)} />
          <Stat label="Dismissed or timed out" value={String(checkIns.ignored)} />
          <Stat label="I'm on fire" value={String(checkIns.onFire)} />
        </dl>
      </Card>

      <section aria-labelledby="days-heading">
        <h2 id="days-heading">Day by day</h2>
        <table className={styles.table}>
          <thead>
            <tr>
              <th scope="col">Day</th>
              <th scope="col">Active time</th>
              <th scope="col">Longest session</th>
              <th scope="col">Average session</th>
              <th scope="col">Switches</th>
            </tr>
          </thead>
          <tbody>
            {review.days.map((d) => (
              <tr key={d.date}>
                <th scope="row">
                  {formatWeekday(d.date)} {formatShortDate(d.date)}
                </th>
                <td>{d.activeMinutes > 0 ? formatDuration(d.activeMinutes) : "—"}</td>
                <td>
                  {d.longestSessionMinutes > 0 ? formatDuration(d.longestSessionMinutes) : "—"}
                </td>
                <td>
                  {d.averageSessionMinutes === null ? "—" : formatDuration(d.averageSessionMinutes)}
                </td>
                <td>{d.contextSwitches > 0 ? d.contextSwitches : "—"}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </section>

      <p className={styles.closing}>{review.reflectiveQuestion}</p>
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
