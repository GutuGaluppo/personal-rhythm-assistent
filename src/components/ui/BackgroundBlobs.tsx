import styles from "./BackgroundBlobs.module.css";

/**
 * The main window's ice background gets 2-3 soft blurred blobs (Design System
 * v0.1 §8) so glass surfaces above (Card, GlassSurface) have something to
 * actually blur. Purely decorative: `aria-hidden`, disabled under Reduce Motion.
 */
export function BackgroundBlobs() {
  return (
    <div className={styles.layer} aria-hidden="true">
      <div className={`${styles.blob} ${styles.aqua}`} />
      <div className={`${styles.blob} ${styles.lilac}`} />
      <div className={`${styles.blob} ${styles.peach}`} />
    </div>
  );
}
