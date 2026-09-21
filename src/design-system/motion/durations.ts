/** Milliseconds. Mirrors IMPLEMENTATION.md §23 and the CSS variables in tokens.css. */
export const MOTION = {
  micro: 190,
  overlay: 240,
  /** One shape change (pause bars -> curve, curve -> serene eyes). */
  transform: 700,
  /** One full breath of the ambient glow. */
  breathe: 4400,
} as const;
