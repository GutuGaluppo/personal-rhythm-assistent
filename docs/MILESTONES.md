# Personal Rhythm Assistant — MILESTONES.md

> Current status (2026-09-22): the functional MVP has been implemented and is being run through a full-day real-world behavioral test. Milestones below remain as technical history/reference. The active delivery track is now **Design Implementation**, defined in `DESIGN_IMPLEMENTATION.md`.

## Milestone 0 — Bootstrap

**Goal:** app opens and the project is healthy.

Deliverables:
- Tauri 2 + React + TypeScript + Vite initialized
- linting/formatting
- Vitest + RTL
- Rust test setup
- SQLite dependency selected
- design tokens from Design System v0.1
- basic main window
- menu bar placeholder
- CI build check

Exit criteria:
- app builds
- app launches
- frontend test passes
- Rust test passes

---

## Milestone 1 — Local Persistence

**Goal:** trustworthy local storage before sensors.

Deliverables:
- SQLite initialization
- migrations
- repositories
- settings table
- activity_events table
- sessions table
- delete-all-local-data command
- retention configuration model

Exit criteria:
- migrations run automatically
- test data can be inserted/read/deleted
- delete-all is covered by tests

---

## Milestone 2 — Activity Sensor

**Goal:** observe neutral local facts.

Deliverables:
- active application sensor
- active/inactive state
- idle sensor
- application switch event
- sensor event bus
- privacy toggle enforcement

Exit criteria:
- active app events appear locally
- idle periods are detected
- switching is detected
- disabling a sensor stops collection
- no content is captured

---

## Milestone 3 — Sessionizer

**Goal:** convert event noise into useful sessions.

Deliverables:
- session boundary rules
- active/idle aggregation
- context-switch counting
- current-session API
- persisted sessions
- synthetic fixtures

Exit criteria:
- deterministic fixtures produce expected sessions
- long idle closes/splits sessions correctly
- switching count is stable

---

## Milestone 4 — Classification + My Day

**Goal:** show a meaningful local picture.

Deliverables:
- app → category mapping
- manual mapping UI
- My Day screen
- current category
- session duration
- active time today
- context switches
- last break

Exit criteria:
- user can remap an app
- My Day updates from real local sessions
- no cloud required

---

## Milestone 5 — Context Engine v0.1

**Goal:** detect evidence without judging.

Deliverables:
- continuous activity detector
- low-idle detector
- rapid switching detector
- Create dominance detector
- evidence model
- combined-signal logic
- developer debug view

Exit criteria:
- each detector is unit-tested
- no detector emits medical/psychological interpretation
- candidate intervention requires >= 2 signals

---

## Milestone 6 — Policy Engine v0.1

**Goal:** keep the app from becoming annoying.

Deliverables:
- cooldown model
- daily intervention count
- silence state
- recent-response state
- `I'm on fire`
- daily ceiling
- retry policy

Exit criteria:
- Continue → 60 min cooldown
- Leave me alone → 60 min cooldown
- max 4/day
- `I'm on fire` suppresses intervention
- silence overrides everything

---

## Milestone 7 — Intervention Window

**Goal:** complete the first meaningful product loop.

Deliverables:
- dedicated Tauri intervention window
- limited capabilities
- Good / Okay / Low
- Continue
- Take a break
- Leave me alone
- timeout/dismiss
- one retry after 30–60 min

Exit criteria:
- window never blocks the main app
- all actions are keyboard accessible
- dismissal is one click
- reason for intervention is visible

---

## Milestone 8 — Pause Mode + Motion

**Goal:** validate the emotional signature.

Deliverables:
- pause screen/window
- 3 / 5 / 10 / custom timer
- Move / Meditate / Nothing
- pause → serene eyes transition
- approved visual state icons
- reduced-motion fallback

Exit criteria:
- timer survives window focus changes
- return flow works
- Reduce Motion uses crossfade instead of transformation

---

## Milestone 9 — Interest Inbox

**Goal:** make forgotten curiosity actionable.

Deliverables:
- add
- list
- archive/delete
- simple suggestion source

Exit criteria:
- capture takes only a few seconds
- items persist locally

---

## Milestone 10 — Daily Summary

**Goal:** close the loop without scoring the user.

Deliverables:
- active time
- category distribution
- longest session
- switching count
- accepted pauses
- optional reflective question

Exit criteria:
- generated from persisted sessions
- no productivity score
- no judgmental copy

---

## Milestone 11 — Weekly Review

**Goal:** reveal trends.

Deliverables:
- rolling 7-day aggregation
- category distribution
- session length trends
- switching patterns
- intervention acceptance/rejection
- deterministic observations
- reflective question

Exit criteria:
- review works with no AI
- observations are explainable from stored data

---

## Milestone 12 — Privacy & Hardening

**Goal:** make local-first trustworthy.

Deliverables:
- privacy/settings screen
- sensor toggles
- retention display
- raw-data deletion
- delete-all-local-data
- capabilities review
- permission descriptions
- app logging review

Exit criteria:
- user can inspect and delete data
- disabled sensors truly stop collection
- intervention surface has minimal privileges
- no sensitive content is logged

---

## Milestone 13 — Behavioral Test Build

**Goal:** use the app for real.

Test period:
- 7–14 days

Observe:
- interventions/day
- accepted vs dismissed
- repeated false positives
- excessive silence
- excessive interruption
- useful pause timing
- user sense of autonomy
- signal thresholds

Do not add major features during this period.

Exit criteria:
- collect qualitative notes
- identify threshold adjustments
- decide whether the product thesis is validated enough for v0.2

---

# v0.2 candidates

Only after behavioral test:

- Apple Calendar
- Focus Anchor
- meal windows
- optional local window title
- keyboard/mouse activity intensity
- exceptional long-flow check
- richer reporting
- Companion/LLM experiment

---

# Milestone discipline

A milestone may start only when the previous milestone's exit criteria are met.

Exception:
UI polishing may run in parallel, but it may not redefine product behavior or delay core validation.


---

# Active track — Design Implementation

The technical milestones above are not the current execution queue unless a behavioral or stability issue discovered during testing requires reopening them.

The active visual milestones are defined in detail in `DESIGN_IMPLEMENTATION.md`.

## D0 — Visual audit of current implementation
Compare the working app against the approved Design System v0.1.

## D1 — Tokens & foundations
Apply palette, typography, spacing, radii, borders, glass, shadows, and motion tokens globally.

## D2 — Official icon system
Integrate the approved icon family and all state variants, including the warm `I'm on fire` state.

## D3 — Core shell
Implement the approved neutral ice background, blobs, sidebar/menu surfaces, window hierarchy, and reusable card patterns.

## D4 — Core screens
Refine My Day, intervention, pause, return flow, and weekly review in high fidelity.

## D5 — Motion & microinteractions
Implement pause → serene eyes, state transitions, overlays, breathing/glow, and Reduce Motion behavior.

## D6 — Responsive desktop states & accessibility
Validate window resizing, density, contrast, keyboard navigation, focus states, and reduced motion.

## D7 — Visual QA in real use
Run the finished design during normal daily use and document friction, excess decoration, visual fatigue, and state legibility.

Design must not modify already validated behavioral logic unless a separate product decision is made.
