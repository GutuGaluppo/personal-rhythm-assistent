import { useEffect } from "react";
import { BackgroundBlobs } from "@/components/ui/BackgroundBlobs";
import { onNavigate } from "@/lib/tauri/events";
import { isPageId, pagesFor, useNavigation } from "@/stores/navigation";
import { Page } from "./router";
import styles from "./App.module.css";

export function App() {
  const { page, go } = useNavigation();
  const pages = pagesFor(import.meta.env.DEV);

  // The menu bar can ask for a specific page.
  useEffect(() => {
    let stop = () => {};
    let cancelled = false;
    void onNavigate((target) => {
      if (isPageId(target)) go(target);
    }).then((unlisten) => {
      if (cancelled) unlisten();
      else stop = unlisten;
    });
    return () => {
      cancelled = true;
      stop();
    };
  }, [go]);
  return (
    <div className={styles.shell}>
      <BackgroundBlobs />
      <nav aria-label="Main" className={styles.nav}>
        <span className={styles.brand}>Personal Rhythm Assistant</span>
        {pages.map((p) => (
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
