import { AppCategories } from "./AppCategories";
import styles from "./Settings.module.css";

export function Settings() {
  return (
    <div className={styles.page}>
      <h1 className={styles.heading}>Settings</h1>
      <section aria-labelledby="apps-heading">
        <h2 id="apps-heading">Apps &amp; categories</h2>
        <p className={styles.muted}>
          Choose what each app counts as. Nothing is guessed: apps stay “Unknown” until you say
          otherwise. A change applies to your current session and to new ones.
        </p>
        <AppCategories />
      </section>
    </div>
  );
}
