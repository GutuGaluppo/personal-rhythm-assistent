import type { ReactNode } from "react";
import styles from "./Card.module.css";

export function Card({
  title,
  children,
  labelledBy,
  className,
}: {
  title: string;
  children: ReactNode;
  labelledBy: string;
  className?: string;
}) {
  const combined = className ? `${styles.card} ${className}` : styles.card;
  return (
    <section className={combined} aria-labelledby={labelledBy}>
      <h2 id={labelledBy} className={styles.title}>
        {title}
      </h2>
      {children}
    </section>
  );
}
