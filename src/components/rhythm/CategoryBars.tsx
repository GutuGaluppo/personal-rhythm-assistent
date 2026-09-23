import { formatDuration, formatPercent } from "@/lib/utils/format";
import type { CategoryShare } from "@/types";
import styles from "./CategoryBars.module.css";

/**
 * Where the time went. The bars are decoration: every share is also written as
 * time and percentage, so nothing depends on seeing them.
 */
export function CategoryBars({ shares }: { shares: CategoryShare[] }) {
  return (
    <ul className={styles.list}>
      {shares.map((c) => (
        <li key={c.category}>
          <div className={styles.row}>
            <span>{c.category}</span>
            <span>
              {formatDuration(c.minutes)} · {formatPercent(c.share)}
            </span>
          </div>
          <div className={styles.track} aria-hidden="true">
            <div className={styles.bar} style={{ width: formatPercent(c.share) }} />
          </div>
        </li>
      ))}
    </ul>
  );
}
