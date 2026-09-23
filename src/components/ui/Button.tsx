import { forwardRef } from "react";
import type { ButtonHTMLAttributes } from "react";
import styles from "./Button.module.css";

type Variant = "primary" | "secondary" | "ghost";

type Props = ButtonHTMLAttributes<HTMLButtonElement> & {
  variant?: Variant;
};

const VARIANT_CLASS: Record<Variant, string> = {
  primary: styles.primary,
  secondary: styles.secondary,
  ghost: styles.ghost,
};

/** Design System v0.1 §6 "Botões e controles": Primary / Secondary / Ghost. */
export const Button = forwardRef<HTMLButtonElement, Props>(function Button(
  { variant = "primary", className, type = "button", ...props },
  ref,
) {
  const combined = className
    ? `${styles.button} ${VARIANT_CLASS[variant]} ${className}`
    : `${styles.button} ${VARIANT_CLASS[variant]}`;
  return <button ref={ref} type={type} className={combined} {...props} />;
});
