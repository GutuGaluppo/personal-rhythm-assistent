import { PAGES, useNavigation } from "@/stores/navigation";
import { Page } from "./router";
import styles from "./App.module.css";

export function App() {
  const { page, go } = useNavigation();
  return (
    <div className={styles.shell}>
      <nav aria-label="Main" className={styles.nav}>
        <span className={styles.brand}>Personal Rhythm Assistant</span>
        {PAGES.map((p) => (
          <button
            key={p.id}
            type="button"
            className={styles.navButton}
            aria-current={page === p.id ? "page" : undefined}
            onClick={() => go(p.id)}
          >
            {p.label}
          </button>
        ))}
      </nav>
      <main className={styles.main}>
        <Page />
      </main>
    </div>
  );
}
