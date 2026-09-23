import normal from "@/design-system/icons/normal.png";
import onFire from "@/design-system/icons/on-fire.png";
import silent from "@/design-system/icons/silent.png";
import styles from "./StateIcon.module.css";

export type AppState = "normal" | "intervention" | "onFire" | "silent";

const SRC: Record<AppState, string> = {
  // No dedicated "intervention" render exists yet (tray only distinguishes
  // default/pause/silent/on-fire, IMPLEMENTATION.md §D2); the calm base icon
  // is the approved stand-in until one is designed.
  normal,
  intervention: normal,
  onFire,
  silent,
};

/**
 * The official glass/serene-eyes mark (Design System v0.1 §9), sized for
 * inline use next to a headline. State is never conveyed by this alone --
 * callers always pair it with a text label.
 */
export function StateIcon({
  state,
  size = 28,
  className,
}: {
  state: AppState;
  size?: number;
  className?: string;
}) {
  const combined = className ? `${styles.icon} ${className}` : styles.icon;
  return (
    <img
      src={SRC[state]}
      alt=""
      aria-hidden="true"
      width={size}
      height={size}
      className={combined}
    />
  );
}
