import styles from "./Settings.module.css";

/** System permissions and network access, in plain words. See docs/PRIVACY_REVIEW.md. */
export function PermissionsNote() {
  return (
    <section aria-labelledby="permissions-heading" className={styles.block}>
      <h2 id="permissions-heading">System permissions</h2>
      <p>
        This app asks macOS for no special permissions. It doesn&apos;t need Accessibility, Screen
        Recording, Input Monitoring or Full Disk Access.
      </p>
      <ul>
        <li>It reads which app is in front, by name only.</li>
        <li>
          It reads how many seconds have passed since your last keyboard or mouse input. It never
          sees what you type or where you click.
        </li>
      </ul>
      <p>It has no network features and needs no internet. Nothing leaves this device.</p>
    </section>
  );
}
