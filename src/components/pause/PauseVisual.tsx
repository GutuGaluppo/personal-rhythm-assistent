import { useEffect } from "react";
import { animate, motion, useMotionTemplate, useMotionValue, useTransform } from "motion/react";
import { MOTION } from "@/design-system/motion/durations";

/**
 * The signature motion (IMPLEMENTATION.md §15):
 *   pause bars -> compress -> curve -> serene eyes -> subtle glow/breathing.
 *
 * Reduce Motion: crossfade only. The bars fade out and the eyes fade in, nothing
 * changes shape or moves, and the glow does not breathe.
 */
type Props = {
  /** false: the pause bars. true: the serene eyes. */
  settled: boolean;
  reduceMotion: boolean;
};

const VIEW_W = 200;
const VIEW_H = 120;
const CY = 60;

export function PauseVisual({ settled, reduceMotion }: Props) {
  return (
    <svg
      viewBox={`0 0 ${VIEW_W} ${VIEW_H}`}
      role="img"
      aria-label={settled ? "Resting" : "Pause"}
      data-motion={reduceMotion ? "crossfade" : "morph"}
      data-settled={settled}
      width="100%"
      style={{ maxWidth: 280 }}
    >
      {reduceMotion ? <Crossfade settled={settled} /> : <Morph settled={settled} />}
    </svg>
  );
}

// ---- reduced: opacity only ---------------------------------------------------------

function Crossfade({ settled }: { settled: boolean }) {
  const fade = { duration: MOTION.overlay / 1000 };
  return (
    <>
      <circle
        cx={VIEW_W / 2}
        cy={CY}
        r={46}
        fill="var(--color-accent)"
        opacity={settled ? 0.25 : 0}
      />
      <motion.g animate={{ opacity: settled ? 0 : 1 }} transition={fade} data-layer="bars">
        <Bar x={82} />
        <Bar x={118} />
      </motion.g>
      <motion.g animate={{ opacity: settled ? 1 : 0 }} transition={fade} data-layer="eyes">
        <EyeArc x={68} />
        <EyeArc x={132} />
      </motion.g>
    </>
  );
}

function Bar({ x }: { x: number }) {
  return (
    <line
      x1={x}
      y1={CY - 34}
      x2={x}
      y2={CY + 34}
      stroke="var(--color-accent)"
      strokeWidth={16}
      strokeLinecap="round"
    />
  );
}

function EyeArc({ x }: { x: number }) {
  return (
    <path
      d={`M ${x - 22} ${CY} Q ${x} ${CY + 12} ${x + 22} ${CY}`}
      fill="none"
      stroke="var(--color-accent)"
      strokeWidth={6}
      strokeLinecap="round"
    />
  );
}

// ---- full motion: one continuous shape change -------------------------------------------

function Morph({ settled }: { settled: boolean }) {
  // 0 = bars, 1 = compressed, 2 = curved eyes.
  const t = useMotionValue(settled ? 2 : 0);

  useEffect(() => {
    const controls = animate(t, settled ? 2 : 0, {
      duration: (MOTION.transform * 2) / 1000,
      ease: "easeInOut",
    });
    return () => controls.stop();
  }, [settled, t]);

  return (
    <>
      <motion.circle
        cx={VIEW_W / 2}
        cy={CY}
        r={46}
        fill="var(--color-accent)"
        initial={false}
        animate={
          settled ? { opacity: [0.14, 0.3, 0.14], scale: [1, 1.08, 1] } : { opacity: 0, scale: 1 }
        }
        transition={
          settled
            ? { duration: MOTION.breathe / 1000, repeat: Infinity, ease: "easeInOut" }
            : { duration: MOTION.overlay / 1000 }
        }
        style={{ transformOrigin: `${VIEW_W / 2}px ${CY}px` }}
      />
      <MorphingEye t={t} side={-1} />
      <MorphingEye t={t} side={1} />
    </>
  );
}

function MorphingEye({ t, side }: { t: ReturnType<typeof useMotionValue<number>>; side: -1 | 1 }) {
  const mid = VIEW_W / 2;
  const useAt = (bars: number, compressed: number, eyes: number) =>
    useTransform(t, [0, 1, 2], [bars, compressed, eyes]);

  const x = useAt(mid + side * 18, mid + side * 26, mid + side * 64);
  const ax = useAt(mid + side * 18, mid + side * 26, mid + side * 64 - 22);
  const bx = useAt(mid + side * 18, mid + side * 26, mid + side * 64 + 22);
  const ay = useAt(CY - 34, CY - 9, CY);
  const by = useAt(CY + 34, CY + 9, CY);
  const cy = useAt(CY, CY, CY + 12);
  const width = useAt(16, 14, 6);
  const d = useMotionTemplate`M ${ax} ${ay} Q ${x} ${cy} ${bx} ${by}`;

  return (
    <motion.path
      d={d}
      fill="none"
      stroke="var(--color-accent)"
      strokeWidth={width}
      strokeLinecap="round"
    />
  );
}
