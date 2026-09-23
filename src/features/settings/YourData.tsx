import { useEffect, useRef, useState, type ReactNode } from "react";
import { Button } from "@/components/ui/Button";
import { formatBytes, formatDuration } from "@/lib/utils/format";
import type { ActivityEvent, TableStats } from "@/types";
import { useDataOverview, useDeleteAll, useDeleteRaw, useRecentEvents } from "./usePrivacy";
import styles from "./Settings.module.css";

const when = (iso: string) =>
  new Date(iso).toLocaleString("en-US", {
    month: "short",
    day: "numeric",
    hour: "numeric",
    minute: "2-digit",
  });

function range(s: TableStats): string {
  if (s.count === 0) return "—";
  const day = (v: string | null) => (v ? v.slice(0, 10) : "");
  return day(s.oldest) === day(s.newest) ? day(s.oldest) : `${day(s.oldest)} to ${day(s.newest)}`;
}

export function describeEvent(e: ActivityEvent): string {
  switch (e.type) {
    case "active_application":
      return `${e.applicationName} in front`;
    case "application_switch":
      return `Switched from ${e.fromBundleId} to ${e.toBundleId}`;
    case "idle":
      return `No input for ${formatDuration(e.seconds / 60)}`;
  }
}

export function YourData() {
  const { data, isPending, isError } = useDataOverview();
  const [showRecent, setShowRecent] = useState(false);

  return (
    <section aria-labelledby="data-heading" className={styles.block}>
      <h2 id="data-heading">Your data</h2>
      <p className={styles.muted}>Exactly what is stored on this device right now.</p>

      {isPending && <p role="status">Loading…</p>}
      {isError && <p role="alert">Couldn&apos;t read what is stored.</p>}
      {data && (
        <table className={styles.table}>
          <thead>
            <tr>
              <th scope="col">Kind</th>
              <th scope="col">Items</th>
              <th scope="col">Dates</th>
            </tr>
          </thead>
          <tbody>
            {(
              [
                ["Raw activity", data.activityEvents],
                ["Sessions", data.sessions],
                ["Check-ins", data.checkIns],
                ["Pauses", data.pauses],
                ["Daily summaries", data.dailySummaries],
                ["Interests", data.interests],
              ] as const
            ).map(([label, stats]) => (
              <tr key={label}>
                <th scope="row">{label}</th>
                <td>{stats.count}</td>
                <td>{range(stats)}</td>
              </tr>
            ))}
            <tr>
              <th scope="row">App categories you chose</th>
              <td>{data.appCategoryOverrides}</td>
              <td>—</td>
            </tr>
            <tr>
              <th scope="row">Size on disk</th>
              <td colSpan={2}>{formatBytes(data.databaseBytes)}</td>
            </tr>
          </tbody>
        </table>
      )}

      <Button
        variant="ghost"
        aria-expanded={showRecent}
        aria-controls="recent-activity"
        onClick={() => setShowRecent(!showRecent)}
      >
        {showRecent ? "Hide recent activity" : "Show recent activity"}
      </Button>
      {showRecent && <Recent />}

      <Deletion />
    </section>
  );
}

function Recent() {
  const { data, isPending, isError } = useRecentEvents(true);
  return (
    <div id="recent-activity">
      <p className={styles.muted}>
        The newest raw events, as stored. This is all that is kept: app names, switches and how long
        you were away.
      </p>
      {isPending && <p role="status">Loading…</p>}
      {isError && <p role="alert">Couldn&apos;t read recent activity.</p>}
      {data && data.length === 0 && <p className={styles.muted}>No raw activity is stored.</p>}
      {data && data.length > 0 && (
        <ul className={styles.events} aria-label="Recent raw activity">
          {data.map((e, i) => (
            <li key={`${e.timestamp}-${i}`}>
              <time dateTime={e.timestamp}>{when(e.timestamp)}</time> · {describeEvent(e)}
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

type Pending = null | "raw" | "all";

/** Focus starts on the safe choice, Escape backs out, and the caller restores focus. */
function Confirm({
  id,
  confirmLabel,
  onConfirm,
  onCancel,
  children,
}: {
  id: string;
  confirmLabel: string;
  onConfirm: () => void;
  onCancel: () => void;
  children: ReactNode;
}) {
  const cancel = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    cancel.current?.focus();
  }, []);
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onCancel();
    };
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  }, [onCancel]);

  return (
    <div role="alertdialog" aria-labelledby={id} className={styles.confirm}>
      <p id={id}>{children}</p>
      <div className={styles.actions}>
        <Button variant="primary" onClick={onConfirm}>
          {confirmLabel}
        </Button>
        <Button ref={cancel} variant="ghost" onClick={onCancel}>
          Cancel
        </Button>
      </div>
    </div>
  );
}

function Deletion() {
  const raw = useDeleteRaw();
  const all = useDeleteAll();
  const [confirming, setConfirming] = useState<Pending>(null);
  const rawTrigger = useRef<HTMLButtonElement>(null);
  const allTrigger = useRef<HTMLButtonElement>(null);

  const close = (which: Exclude<Pending, null>) => {
    setConfirming(null);
    // Back to the button that opened the question.
    (which === "raw" ? rawTrigger : allTrigger).current?.focus();
  };

  return (
    <div className={styles.danger}>
      <h3>Delete</h3>
      <div className={styles.actions}>
        <Button ref={rawTrigger} variant="secondary" onClick={() => setConfirming("raw")}>
          Delete raw activity…
        </Button>
        <Button ref={allTrigger} variant="secondary" onClick={() => setConfirming("all")}>
          Delete all local data…
        </Button>
      </div>

      {confirming === "raw" && (
        <Confirm
          id="confirm-raw"
          confirmLabel="Yes, delete raw activity"
          onCancel={() => close("raw")}
          onConfirm={() => raw.mutate(undefined, { onSettled: () => close("raw") })}
        >
          Delete the raw app and idle events? Sessions, summaries, check-ins and interests stay.
          This can&apos;t be undone.
        </Confirm>
      )}

      {confirming === "all" && (
        <Confirm
          id="confirm-all"
          confirmLabel="Yes, delete everything"
          onCancel={() => close("all")}
          onConfirm={() => all.mutate(undefined, { onSettled: () => close("all") })}
        >
          Delete everything this app has stored on this device: activity, sessions, check-ins,
          summaries, interests and settings? This can&apos;t be undone. Your Reduce motion choice is
          kept.
        </Confirm>
      )}

      {raw.isSuccess && (
        <p role="status">
          Deleted {raw.data} raw {raw.data === 1 ? "event" : "events"}.
        </p>
      )}
      {all.isSuccess && <p role="status">Everything was deleted.</p>}
      {(raw.isError || all.isError) && (
        <p role="alert">That didn&apos;t work. Nothing was changed.</p>
      )}
    </div>
  );
}
