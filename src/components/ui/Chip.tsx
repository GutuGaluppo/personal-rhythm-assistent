import { forwardRef } from "react";
import type { ButtonHTMLAttributes } from "react";
import styles from "./Chip.module.css";

type Variant = "default" | "active" | "subtle";

type Props = ButtonHTMLAttributes<HTMLButtonElement> & {
  variant?: Variant;
};

const VARIANT_CLASS: Record<Variant, string> = {
  default: styles.default,
  active: styles.active,
  subtle: styles.subtle,
};

/** Design System v0.1 §6: "Chip padrão / Chip ativo / Chip sutil." */
export const Chip = forwardRef<HTMLButtonElement, Props>(function Chip(
  { variant = "default", className, type = "button", ...props },
  ref,
) {
  const combined = className
    ? `${styles.chip} ${VARIANT_CLASS[variant]} ${className}`
    : `${styles.chip} ${VARIANT_CLASS[variant]}`;
  return <button ref={ref} type={type} className={combined} {...props} />;
});
