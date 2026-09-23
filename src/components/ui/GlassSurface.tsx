import type { ElementType, ComponentPropsWithoutRef } from "react";
import styles from "./GlassSurface.module.css";

type Props<T extends ElementType> = {
  as?: T;
  className?: string;
} & Omit<ComponentPropsWithoutRef<T>, "as" | "className">;

/**
 * The generic translucent/blurred panel (Design System v0.1 §5 "Painel"): for
 * anything that isn't `Card`-shaped -- overlays, the app shell nav, big panels.
 * Visually subtle until something colorful sits behind it to blur (blobs, D3).
 */
export function GlassSurface<T extends ElementType = "div">({ as, className, ...props }: Props<T>) {
  const Tag = as ?? "div";
  const combined = className ? `${styles.surface} ${className}` : styles.surface;
  return <Tag className={combined} {...props} />;
}
