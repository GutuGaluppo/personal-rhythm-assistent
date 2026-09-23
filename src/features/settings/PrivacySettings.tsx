import { NOT_AVAILABLE, SENSORS, days } from "./privacyCatalog";
import { usePrivacyToggles, useRetentionPolicy, useSetToggles } from "./usePrivacy";
import styles from "./Settings.module.css";

export function PrivacySettings() {
  const { data: toggles, isPending, isError } = usePrivacyToggles();
  const { data: retention } = useRetentionPolicy();
  const set = useSetToggles();

  return (
    <section aria-labelledby="privacy-heading" className={styles.block}>
      <h2 id="privacy-heading">Privacy</h2>
      <p className={styles.muted}>
        Everything stays on this device. Nothing is sent anywhere. Turn off anything you don&apos;t
        want collected; it stops right away.
      </p>

      {isPending && <p role="status">Loading…</p>}
      {isError && <p role="alert">Couldn&apos;t read your privacy settings.</p>}

      {toggles &&
        SENSORS.map((sensor) => {
          const on = toggles[sensor.key];
          const id = `sensor-${sensor.key}`;
          return (
            <div key={sensor.key} className={styles.card}>
              <label htmlFor={id} className={styles.switch}>
                <input
                  id={id}
                  type="checkbox"
                  role="switch"
                  checked={on}
                  disabled={set.isPending}
                  onChange={(e) => set.mutate({ ...toggles, [sensor.key]: e.target.checked })}
                />
                <span>{sensor.label}</span>
                <span className={styles.state}>{on ? "On" : "Off"}</span>
              </label>
              <dl className={styles.facts}>
                <dt>What is collected</dt>
                <dd>{sensor.collected}</dd>
                <dt>Why</dt>
                <dd>{sensor.why}</dd>
                <dt>Where it is stored</dt>
                <dd>{sensor.storedIn}</dd>
                <dt>How long it is kept</dt>
                <dd>{retention ? days(retention[sensor.keptFor]) : "…"}</dd>
                <dt>How to delete it</dt>
                <dd>{sensor.deleteWith}</dd>
              </dl>
              {!on && (
                <p className={styles.muted}>
                  Nothing new is collected. What is already stored stays until you delete it.
                </p>
              )}
            </div>
          );
        })}
      {set.isError && <p role="alert">Couldn&apos;t save that change.</p>}

      <h3>Not available yet</h3>
      <ul className={styles.plain}>
        {NOT_AVAILABLE.map((item) => {
          const id = `unavailable-${item.key}`;
          return (
            <li key={item.key} className={styles.card}>
              <label htmlFor={id} className={styles.switch}>
                <input id={id} type="checkbox" role="switch" checked={false} disabled readOnly />
                <span>{item.label}</span>
                <span className={styles.state}>Off · not available</span>
              </label>
              <p className={styles.muted}>{item.note}</p>
            </li>
          );
        })}
      </ul>
    </section>
  );
}
