# Personal Rhythm Assistant — PRIVACY_REVIEW.md

> Status: Milestone 12 review. Everything here describes what the code does today
> and is checked by tests where a test can check it (see "How this is kept true").

## 1. What is collected

Each row is a toggle in Settings → Privacy. All of it stays on this device, in one
local SQLite file. Nothing is sent anywhere.

| Data | What exactly | Why | Kept for (default) |
| --- | --- | --- | --- |
| Active application | The name and bundle id of the app in front (for example `Code`, `com.microsoft.VSCode`) and the moment it changes. A heartbeat repeats it every 5 minutes while it stays in front. | To notice app switching and long stretches. | 7 days |
| Idle detection | How many seconds since the last keyboard or mouse input, and the length of each finished idle period. Only elapsed time. | To tell time at the keyboard from time away. | 7 days |
| Active time | Nothing new. It turns the two above into sessions: start, end, active and idle minutes, switch count, category. | To show your day and notice long sessions. | 30 days |

Derived from those, and kept on this device as well: check-ins shown and how they
were answered (30 days), pauses started (30 days), one summary per finished day
(indefinitely unless you set a limit), the interests you write down (until you
delete them), the category you chose for each app, and your settings.

Not available yet, shown as "off" and impossible to switch on: window title,
keyboard and mouse rhythm, calendar, cloud processing.

## 2. What is never collected

Keystrokes, what is typed, screenshots or screen contents, window titles, documents,
emails or messages, full URLs, which keys or where the pointer was. The event model
has no field that could hold any of these (a test checks the fields).

## 3. System permissions

The app asks for none. It does not need Accessibility, Screen Recording, Input
Monitoring, Full Disk Access, Calendar or Contacts access.

- Which app is in front comes from `NSWorkspace.frontmostApplication` (name and bundle id only).
- Idle time comes from `CGEventSourceSecondsSinceLastEventType`, a counter of seconds
  since the last input event of any type. It does not observe or record events.

## 4. Network

The app has no network features and needs no internet.

- Its own dependencies include no HTTP client, and the build graph for this platform
  contains no HTTP, TLS or socket crate. `scripts/check-no-network.sh` verifies it and
  runs as part of `npm run check` and in CI. (`Cargo.lock` mentions `reqwest` and
  `hyper` for other platforms; they are not part of this build.)
- The Content Security Policy allows only the app's own origin: no remote origin can
  be contacted or loaded.

## 5. Windows and what each may do

Tauri permissions are per window. A window can call a command only if its capability
file grants it, and `build.rs` lists every command so nothing is allowed by default.

**Main window** (`capabilities/default.json`). It draws the screens. From Tauri core it
has only the ability to listen for the menu bar's "show this page" request.

- `core:event:default`
- `allow-delete-all-local-data`
- `allow-get-retention-policy`
- `allow-set-retention-policy`
- `allow-get-privacy-toggles`
- `allow-set-privacy-toggles`
- `allow-get-sensor-state`
- `allow-get-current-session`
- `allow-get-my-day`
- `allow-list-app-mappings`
- `allow-set-app-category`
- `allow-reset-app-category`
- `allow-get-context-assessment`
- `allow-get-policy-view`
- `allow-set-silence`
- `allow-set-on-fire`
- `allow-get-policy-debug`
- `allow-debug-show-intervention`
- `allow-open-pause`
- `allow-list-interests`
- `allow-add-interest`
- `allow-archive-interest`
- `allow-restore-interest`
- `allow-delete-interest`
- `allow-get-interest-suggestion`
- `allow-get-daily-summary`
- `allow-list-summary-days`
- `allow-save-reflection`
- `allow-get-weekly-review`
- `allow-delete-raw-data`
- `allow-get-data-overview`
- `allow-list-recent-activity-events`

**Check-in window** (`capabilities/intervention.json`). It can read the current
check-in, answer it, or dismiss it. Nothing from core, and no other command.

- `allow-get-current-intervention`
- `allow-answer-intervention`
- `allow-dismiss-intervention`

**Pause window** (`capabilities/pause.json`). It can read the pause, start it and end it.
Nothing from core, and no other command.

- `allow-get-pause-view`
- `allow-start-pause`
- `allow-end-pause`

## 6. Logging

The Rust code has exactly two log lines, both printing the error Tauri returns when a
small window cannot be opened. Nothing that identifies what you were doing (app names,
bundle ids, times, text) is ever logged, and the frontend does not use `console`.
A test lists every logging call and fails if a new one appears, so adding one is a
deliberate, reviewed change.

## 7. Deleting data

Settings → Your data:

- **Delete raw activity** removes the app and idle events. Sessions, summaries,
  check-ins and interests stay.
- **Delete all local data** removes everything the app stored, including settings,
  category choices, interests and summaries, and starts fresh. The file is compacted
  and deleted content is overwritten, so it does not linger in the database file.
- **Retention** can be changed at any time. It applies immediately and again every hour,
  and the summary of a finished day is written before its sessions are pruned.

Turning a sensor off stops collection at once (a sensor that is off is not even
queried); data already stored stays until you delete it.

The one thing kept outside the database is the Reduce motion switch, an accessibility
preference stored by the app's webview. Delete-all leaves it alone on purpose.

## 8. How this is kept true

| Claim | Checked by |
| --- | --- |
| Disabled sensors are never queried and record nothing | `tests/core/sensors.rs` |
| No field can hold content | `tests/core/sensors.rs` |
| Deleted content leaves the database file | `tests/core/persistence.rs`, `tests/core/privacy.rs` |
| Windows have exactly the permissions above | `tests/core/capabilities.rs`, `tests/core/hardening.rs` |
| No logging beyond the two lines | `tests/core/hardening.rs` |
| No network code, strict CSP | `scripts/check-no-network.sh`, `tests/core/hardening.rs` |
| The permission lists in this document are current | `tests/core/hardening.rs` |
