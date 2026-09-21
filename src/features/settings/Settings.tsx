import { useState } from "react";
import { readMotionPreference, writeMotionPreference } from "@/lib/utils/motion";
import { AppCategories } from "./AppCategories";
import styles from "./Settings.module.css";

export function Settings() {
  return (
    <div className={styles.page}>
      <h1 className={styles.heading}>Settings</h1>
      <MotionSetting />
      <section aria-labelledby="apps-heading">
        <h2 id="apps-heading">Apps &amp; categories</h2>
        <p className={styles.muted}>
          Choose what each app counts as. Nothing is guessed: apps stay “Unknown” until you say
          otherwise. A change applies to your current session and to new ones.
        </p>
        <AppCategories />
      </section>
    </div>
  );
}

function MotionSetting() {
  const [reduce, setReduce] = useState(() => readMotionPreference() === "reduce");
  return (
    <section aria-labelledby="motion-heading">
      <h2 id="motion-heading">Motion</h2>
      <label>
        <input
          type="checkbox"
          checked={reduce}
          onChange={(e) => {
            setReduce(e.target.checked);
            writeMotionPreference(e.target.checked ? "reduce" : "system");
          }}
        />{" "}
        Reduce motion
      </label>
      <p className={styles.muted}>
        Animations become simple fades. Your system&apos;s Reduce Motion setting is always
        respected, whatever you choose here.
      </p>
    </section>
  );
}
