import { useQuery } from "@tanstack/react-query";
import { getContextAssessment, getPolicyDebug } from "@/lib/tauri/commands";
import type { ContextAssessment, PolicyDebug, PolicyDecision, Signal, SignalKind } from "@/types";
import styles from "./ContextDebug.module.css";

const SIGNAL_LABEL: Record<SignalKind, string> = {
  continuous_activity: "A · Continuous activity",
  insufficient_idle: "B · Little inactivity",
  rapid_switching: "C · Rapid app switching",
  create_dominance: "D · Create share across days",
};

export function formatMeasure(value: number, unit: Signal["evidence"]["unit"]): string {
  switch (unit) {
    case "minutes":
      return `${Math.round(value)} min`;
    case "ratio":
      return `${Math.round(value * 100)}%`;
    case "per_hour":
      return `${value.toFixed(1)} / h`;
  }
}

export function ContextDebug() {
  const { data, isPending, isError } = useQuery({
    queryKey: ["context-assessment"],
    queryFn: getContextAssessment,
    refetchInterval: 10_000,
  });

  return (
    <div className={styles.page}>
      <h1 className={styles.heading}>Context (developer)</h1>
      <p className={styles.muted}>
        Developer view of the signals and the numbers behind them. It is not part of the product.
      </p>
      {isPending && <p role="status">Loading…</p>}
      {isError && <p role="alert">Couldn't read the assessment.</p>}
      {data && <Assessment assessment={data} />}
      <PolicySection />
    </div>
  );
}

function Assessment({ assessment }: { assessment: ContextAssessment }) {
  return (
    <>
      <p>
        {assessment.activeSignalCount} of {assessment.signals.length} signals met ·{" "}
        <strong>
          {assessment.decision === "candidate_intervention" ? "Candidate" : "Observing"}
        </strong>
      </p>
      <table className={styles.table}>
        <caption className={styles.caption}>Signals at {assessment.assessedAt}</caption>
        <thead>
          <tr>
            <th scope="col">Signal</th>
            <th scope="col">Status</th>
            <th scope="col">Measured</th>
            <th scope="col">Threshold</th>
            <th scope="col">Evidence</th>
          </tr>
        </thead>
        <tbody>
          {assessment.signals.map((s) => (
            <tr key={s.kind}>
              <th scope="row">{SIGNAL_LABEL[s.kind]}</th>
              <td>{s.active ? "Met" : "Not met"}</td>
              <td>{formatMeasure(s.evidence.measured, s.evidence.unit)}</td>
              <td>{formatMeasure(s.evidence.threshold, s.evidence.unit)}</td>
              <td>{s.evidence.explanation}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </>
  );
}

export function describeDecision(d: PolicyDecision): string {
  switch (d.kind) {
    case "observe":
      return "Observe: allowed to speak, but the evidence isn't there.";
    case "ask_checkin":
      return d.is_retry ? "Ask a check-in (the single retry)." : "Ask a check-in.";
    case "silent":
      switch (d.reason.kind) {
        case "silent_mode":
          return "Silent: silence is on.";
        case "cooldown":
          return `Silent: cooldown until ${d.reason.until}.`;
        case "daily_ceiling":
          return `Silent: daily limit of ${d.reason.limit} reached.`;
        case "on_fire":
          return `Silent: "I'm on fire" until ${d.reason.until}.`;
      }
  }
}

function PolicySection() {
  const { data } = useQuery({
    queryKey: ["policy-debug"],
    queryFn: getPolicyDebug,
    refetchInterval: 10_000,
  });
  if (!data) return null;
  return <Policy debug={data} />;
}

function Policy({ debug }: { debug: PolicyDebug }) {
  const { view, decision } = debug;
  return (
    <section aria-labelledby="policy-heading">
      <h2 id="policy-heading">Policy</h2>
      <p>
        <strong>{describeDecision(decision)}</strong>
      </p>
      <dl className={styles.facts}>
        <dt>Silence</dt>
        <dd>{view.silent ? "On" : "Off"}</dd>
        <dt>I&apos;m on fire until</dt>
        <dd>{view.onFireUntil ?? "—"}</dd>
        <dt>Cooldown until</dt>
        <dd>{view.cooldownUntil ?? "—"}</dd>
        <dt>Check-ins today</dt>
        <dd>
          {view.shownToday} of {view.dailyLimit}
        </dd>
        <dt>Retry pending</dt>
        <dd>{view.retryPending ? "Yes" : "No"}</dd>
        <dt>Frequency question due</dt>
        <dd>{view.frequencyPromptDue ? "Yes" : "No"}</dd>
      </dl>
    </section>
  );
}
