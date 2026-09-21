import { CATEGORIES, type AppMapping, type Category, type MappingSource } from "@/types";
import { useAppMappings, useResetAppCategory, useSetAppCategory } from "./useAppMappings";
import styles from "./Settings.module.css";

const SOURCE_LABEL: Record<MappingSource, string> = {
  user: "Your choice",
  default: "Default",
  unset: "Not set",
};

export function AppCategories() {
  const { data, isPending, isError } = useAppMappings();
  const setCategory = useSetAppCategory();
  const reset = useResetAppCategory();

  if (isPending) return <p role="status">Loading…</p>;
  if (isError) return <p role="alert">Couldn't read the app list from this device.</p>;
  if (data.length === 0) {
    return <p className={styles.muted}>Apps you use will show up here once they've been seen.</p>;
  }

  return (
    <ul className={styles.list}>
      {data.map((app) => (
        <Row
          key={app.bundleId}
          app={app}
          onChange={(category) => setCategory.mutate({ bundleId: app.bundleId, category })}
          onReset={() => reset.mutate(app.bundleId)}
        />
      ))}
    </ul>
  );
}

function Row({
  app,
  onChange,
  onReset,
}: {
  app: AppMapping;
  onChange: (category: Category) => void;
  onReset: () => void;
}) {
  const selectId = `category-${app.bundleId}`;
  return (
    <li className={styles.row}>
      <div>
        <div className={styles.appName}>{app.applicationName}</div>
        <div className={styles.muted}>
          {app.bundleId} · {SOURCE_LABEL[app.source]}
        </div>
      </div>
      <div className={styles.controls}>
        <label htmlFor={selectId} className={styles.visuallyHidden}>
          Category for {app.applicationName}
        </label>
        <select
          id={selectId}
          value={app.category}
          onChange={(e) => onChange(e.target.value as Category)}
        >
          {CATEGORIES.map((c) => (
            <option key={c} value={c}>
              {c}
            </option>
          ))}
        </select>
        {app.source === "user" && (
          <button
            type="button"
            aria-label={`Reset ${app.applicationName} to default`}
            onClick={onReset}
          >
            Reset
          </button>
        )}
      </div>
    </li>
  );
}
