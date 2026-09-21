import type { ReactNode } from "react";
import styles from "./Card.module.css";

export function Card({
  title,
  children,
  labelledBy,
}: {
  title: string;
  children: ReactNode;
  labelledBy: string;
}) {
  return (
    <section className={styles.card} aria-labelledby={labelledBy}>
      <h2 id={labelledBy} className={styles.title}>
        {title}
      </h2>
      {children}
    </section>
  );
}
