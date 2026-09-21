# Personal Rhythm Assistant — IMPLEMENTATION.md

> Status: MVP v0.1 frozen scope  
> Platform: macOS-first  
> Desktop shell: Tauri 2  
> Frontend: React + TypeScript + Vite  
> Core: Rust  
> Persistence: SQLite  
> State: Zustand + TanStack Query  
> Motion: Motion + SVG  
> Principle: local-first, optional cloud later

---

## 1. Product goal

Build a local desktop companion that observes application activity and idle patterns, groups them into work sessions, recognizes sustained or fragmented focus, and gently offers optional breaks without blocking, judging, or gamifying the user.

The MVP must prove one thing:

> The app can understand enough of the user's work rhythm to make timely, useful, non-intrusive interventions while preserving autonomy.

---

## 2. Non-negotiable product rules

1. Never block applications or user actions.
2. Every intervention must be dismissible in one click.
3. Always allow the user to continue working.
4. Never use streaks, red badges, productivity scores, guilt, punishment, or artificial urgency.
5. Never infer burnout, anxiety, depression, addiction, laziness, discipline, or moral quality of time use.
6. No keylogging.
7. No screenshots.
8. No email/message/document content inspection.
9. No full URL capture.
10. Local-first by default.
11. LLMs never decide whether to interrupt.
12. Context Engine detects; Policy Engine decides whether the app may appear.
13. Normal behavior target: 2–3 interventions/day.
14. Hard experimental ceiling: 4 interventions/day.
15. `Continue` and `Leave me alone` create a 60-minute cooldown.
16. `I'm on fire` suppresses normal interventions for 120 minutes.
17. Reduce Motion must be supported from the first release.

---

## 3. MVP technical scope

### Must-have

- macOS desktop app
- background execution
- menu bar / tray
- main window
- dedicated intervention window
- local SQLite database
- active application sensor
- idle detection
- application switching detection
- sessionizer
- manual activity/category mapping
- Context Engine v0.1
- Policy Engine v0.1
- intervention flow
- pause timer
- `I'm on fire`
- Interest Inbox
- daily summary
- 7-day review
- privacy/settings screen
- local-data deletion
- Reduce Motion

### Should-have after core validation

- Apple Calendar integration
- Focus Anchor
- configurable meal windows
- optional local window-title capture
- keyboard/mouse activity intensity
- exceptional check during long `I'm on fire`
- richer reporting

### Later

- LLM Companion
- Google Calendar
- guided audio library
- automatic project detection
- cloud sync
- multi-device
- Apple Watch
- Windows/Linux production support

---

## 4. Architecture

```text
┌───────────────────────────────────────────────────────┐
│ UI Layer                                              │
│                                                       │
│ Main Window   Menu Bar   Intervention   Pause Window  │
│ React         React      React           React         │
└──────────────────────┬────────────────────────────────┘
                       │
                  Tauri commands/events
                       │
┌──────────────────────▼────────────────────────────────┐
│ Rust Application Core                                 │
│                                                       │
│ Scheduler                                             │
│ Sessionizer                                           │
│ Context Engine                                        │
│ Policy Engine                                         │
│ Intervention Manager                                  │
│ Interest Inbox                                        │
│ Reports                                               │
└───────────────┬──────────────────┬────────────────────┘
                │                  │
        ┌───────▼────────┐  ┌──────▼─────────┐
        │ Sensors        │  │ Persistence    │
        │ active app     │  │ SQLite         │
        │ idle           │  │ preferences    │
        │ app switching  │  │ summaries      │
        │ system state   │  │ feedback       │
        └───────┬────────┘  └────────────────┘
                │
        ┌───────▼─────────┐
        │ macOS adapters  │
        │ Swift/native    │
        │ only if needed  │
        └─────────────────┘
```

---

## 5. Frontend stack

### Core

- React
- TypeScript
- Vite
- Zustand
- TanStack Query
- Motion
- CSS Modules or Tailwind
- Radix primitives where useful

### Rules

- No Next.js.
- No heavy design-system framework.
- Do not let a component library redefine the approved visual direction.
- Design System v0.1 remains the visual source of truth.
- All motion must have a reduced-motion fallback.

---

## 6. Rust core modules

Recommended module boundaries:

```text
src-tauri/src/
├── app/
│   ├── mod.rs
│   ├── commands.rs
│   └── events.rs
├── sensors/
│   ├── mod.rs
│   ├── active_app.rs
│   ├── idle.rs
│   ├── app_switch.rs
│   └── system_state.rs
├── sessions/
│   ├── mod.rs
│   ├── model.rs
│   └── sessionizer.rs
├── context/
│   ├── mod.rs
│   ├── signals.rs
│   ├── detectors.rs
│   └── engine.rs
├── policy/
│   ├── mod.rs
│   ├── rules.rs
│   └── engine.rs
├── interventions/
│   ├── mod.rs
│   ├── model.rs
│   └── manager.rs
├── persistence/
│   ├── mod.rs
│   ├── db.rs
│   ├── migrations.rs
│   └── repositories/
├── reports/
│   ├── mod.rs
│   ├── daily.rs
│   └── weekly.rs
├── privacy/
│   ├── mod.rs
│   ├── retention.rs
│   └── permissions.rs
└── platform/
    └── macos/
```

Do not merge the Context Engine and Policy Engine.

---

## 7. Event model

Raw sensors emit neutral facts only.

Example:

```ts
type ActivityEvent =
  | {
      type: "active_application";
      timestamp: string;
      bundleId: string;
      applicationName: string;
    }
  | {
      type: "idle";
      timestamp: string;
      seconds: number;
    }
  | {
      type: "application_switch";
      timestamp: string;
      fromBundleId: string;
      toBundleId: string;
    };
```

Sensors must never emit interpretations such as:

```text
user_is_overworking = true
user_is_anxious = true
```

---

## 8. Session model

The Sessionizer converts raw events into compact work sessions.

```ts
type Session = {
  id: string;
  startedAt: string;
  endedAt?: string;

  activeMinutes: number;
  idleMinutes: number;
  contextSwitches: number;

  category:
    | "Create"
    | "Learn"
    | "Explore"
    | "Move"
    | "Life"
    | "People"
    | "Recover"
    | "Think"
    | "Unknown";

  applicationIds: string[];
  projectId?: string;
};
```

The Context Engine should primarily consume sessions, not raw events.

---

## 9. Activity classification v0.1

Classification is manual/default-based.

Example:

```text
VS Code      → Create
Terminal     → Create
Figma        → Create
Spotify      → Recover
Books        → Learn
Firefox      → Unknown / user-defined
```

Users must be able to change mappings.

No AI classification in v0.1.

---

## 10. Context Engine v0.1

The MVP only needs four primary signals.

### Signal A — continuous activity

Initial threshold:

```text
>= 90 minutes active
```

### Signal B — insufficient real idle

Long sessions with little meaningful inactivity.

### Signal C — rapid context switching

High frequency of application/project switching inside a session.

Threshold must be configurable during development.

### Signal D — Create dominance

Create occupies a large proportion of active time, especially across multiple days.

### Additional evidence stored but not required initially

- ignored break suggestions
- late work
- repeated long sessions
- category neglect

### Decision rule

Never intervene based on one isolated signal.

Initial heuristic:

```text
if active_signals >= 2:
    candidate_intervention
else:
    observe
```

Avoid a single numeric "ultra productivity score".

Use discrete signals + evidence.

---

## 11. Policy Engine v0.1

The Policy Engine decides whether a candidate intervention may surface.

Pseudo-code:

```ts
if (silentMode) return "SILENT";

if (cooldownActive) return "SILENT";

if (interventionsToday >= 4) return "SILENT";

if (onFireMode) return "SILENT";

if (activeSignals < 2) return "OBSERVE";

return "ASK_CHECKIN";
```

Target behavior:

```text
normal: 2–3 interventions/day
absolute experimental ceiling: 4/day
```

The Policy Engine owns:

- cooldown
- daily limit
- `I'm on fire`
- silence
- recent response
- intervention retry policy

---

## 12. Intervention flow

Initial overlay:

```text
You've been active for 96 minutes.
How is your energy right now?

[ Good ] [ Okay ] [ Low ]

Leave me alone for now
```

If Good/Okay:

```text
[ Continue ] [ Take a break ]
```

If Low:

```text
[ Move ] [ Meditate ] [ Do nothing ]
```

Rules:

- non-modal
- small dedicated window
- always dismissible
- visible roughly 20 seconds
- if ignored: one retry after 30–60 minutes
- never immediately repeat identical wording

---

## 13. Cooldown rules

```text
Continue            → 60 min
Leave me alone      → 60 min
Accepted break      → no immediate follow-up
3 refusals          → ask whether intervention frequency should be reduced
```

---

## 14. I'm on fire

Activation:

- keyboard shortcut
- menu bar
- response from intervention

Duration:

```text
120 min
```

Behavior:

- suppress normal interventions
- switch to approved warm-state icon
- no urgency language
- no animated flame
- no additional gamification

The exceptional 180-minute check is postponed to v0.2.

---

## 15. Pause Mode v0.1

Durations:

```text
3 min
5 min
10 min
custom
```

Pause types:

- silence
- meditation
- walking
- stretching

No audio library required in v0.1.

The app may show a minimal prompt such as:

> Close your eyes for a moment and follow your breathing.

### Signature motion

```text
pause bars
→ compress
→ curve
→ serene eyes
→ subtle glow/breathing
```

Reduced Motion:

```text
crossfade only
```

---

## 16. Main surfaces

### Main Window

Initial navigation:

- My Day
- Interest Inbox
- History
- Settings

### Menu Bar

Minimum commands:

```text
How is my rhythm?
Take a break
I'm on fire
Interest Inbox
Open app
Silence
```

### Intervention Window

Limited capability surface.

May only:

- read current intervention
- submit answer
- dismiss

It must not have broad app permissions.

---

## 17. My Day v0.1

Display:

- current session duration
- current category
- active time today
- context switches
- last real break
- intervention count
- optional current project

Calendar data is omitted until the Calendar milestone.

---

## 18. Interest Inbox v0.1

Simple CRUD.

Example:

```text
+ Learn WebGPU
+ Restore bicycle frame
+ Read about granular synthesis
```

Fields:

```text
id
text
created_at
archived_at?
```

No tagging system required in v0.1.

---

## 19. Daily summary

Show:

- total active time
- category distribution
- longest continuous session
- context switches
- accepted breaks

Optional question:

> How did today's rhythm feel to you?

No productivity score.

---

## 20. Weekly review

Period:

```text
last 7 days
```

Show:

- category distribution
- long-session pattern
- context switching
- accepted/rejected interventions
- neglected intentions, if available

Deterministic observations only.

Example:

```text
Create occupied more space this week.
Your longest session was 3h 14m.
You accepted 4 of 7 break suggestions.
```

Close with:

> Does anything here feel different from what you would like?

---

## 21. SQLite schema — initial proposal

```sql
settings
activity_events
sessions
app_category_mappings
interventions
intervention_feedback
daily_summaries
interests
weekly_intentions
```

Suggested retention defaults:

```text
activity_events  → 7 days
sessions         → 30 days
daily_summaries  → indefinite
```

The user must be able to:

- change future retention policy
- delete raw data
- delete all local data

---

## 22. Privacy settings

Required from v0.1.

Each tracked category must explain:

```text
what is collected
why it is collected
where it is stored
retention
delete option
```

Initial toggles:

```text
Active application       ON
Active time              ON
Idle detection           ON
Window title             OFF
Keyboard/mouse rhythm    OFF
Calendar                 OFF
Cloud processing         OFF
```

The last three can remain non-functional placeholders until their milestones,
but must not pretend to be active.

---

## 23. Motion rules

Primary motion language:

```text
breathe
transform
settle
```

Avoid:

- bounce-heavy motion
- attention-seeking loops
- countdown pressure
- rapid pulsing
- red warning states

Typical durations:

```text
micro feedback        160–220 ms
overlay entry         220–260 ms
state transform       500–900 ms
ambient breathing     3.8–5 s
background blobs      10–16 s
```

---

## 24. Accessibility

MVP requirements:

- full keyboard navigation
- visible focus states
- semantic controls
- adequate contrast
- Reduce Motion
- no meaning conveyed only by color
- intervention actions reachable without pointer
- icon states also labeled in accessible text

---

## 25. Folder structure

```text
personal-rhythm-assistant/
├── docs/
│   ├── IMPLEMENTATION.md
│   └── MILESTONES.md
│
├── src/
│   ├── app/
│   │   ├── App.tsx
│   │   ├── router.tsx
│   │   └── providers.tsx
│   │
│   ├── components/
│   │   ├── ui/
│   │   ├── rhythm/
│   │   ├── intervention/
│   │   └── pause/
│   │
│   ├── features/
│   │   ├── my-day/
│   │   ├── interest-inbox/
│   │   ├── history/
│   │   ├── settings/
│   │   ├── intervention/
│   │   ├── pause/
│   │   └── reports/
│   │
│   ├── design-system/
│   │   ├── tokens/
│   │   ├── motion/
│   │   ├── icons/
│   │   └── components/
│   │
│   ├── hooks/
│   ├── lib/
│   │   ├── tauri/
│   │   ├── query/
│   │   └── utils/
│   ├── stores/
│   ├── types/
│   └── main.tsx
│
├── src-tauri/
│   ├── capabilities/
│   ├── migrations/
│   └── src/
│       ├── app/
│       ├── sensors/
│       ├── sessions/
│       ├── context/
│       ├── policy/
│       ├── interventions/
│       ├── persistence/
│       ├── reports/
│       ├── privacy/
│       └── platform/
│           └── macos/
│
├── tests/
│   ├── frontend/
│   ├── core/
│   ├── integration/
│   └── fixtures/
│
├── scripts/
├── .github/
│   └── workflows/
├── package.json
├── vite.config.ts
├── tsconfig.json
├── README.md
└── .gitignore
```

---

## 26. Testing strategy

### Frontend

- Vitest
- React Testing Library

Test:

- intervention choices
- pause states
- settings toggles
- visual state logic
- reduced motion behavior

### Rust

Test:

- session boundaries
- active/idle aggregation
- switching count
- signal detectors
- policy cooldown
- daily ceiling
- `I'm on fire`

### Integration

Use synthetic fixtures.

Example:

```text
09:00 VS Code
09:30 Terminal
09:31 VS Code
10:30 VS Code
10:31 Terminal
10:40 VS Code
```

Expected:

```text
continuous_activity = true
switching_signal = true
candidate_intervention = true
```

Do not rely only on real-world manual testing.

---

## 27. Definition of Done — MVP v0.1

The MVP is ready for behavioral testing when all are true:

- [ ] launches on macOS
- [ ] runs in background
- [ ] menu bar works
- [ ] active app is detected
- [ ] idle is detected
- [ ] app switches are counted
- [ ] sessions are created
- [ ] apps can be manually mapped to categories
- [ ] continuous activity is detected
- [ ] at least 2 signals can combine
- [ ] Policy Engine respects cooldown
- [ ] Policy Engine respects daily limit
- [ ] intervention window works
- [ ] user can continue
- [ ] user can request silence
- [ ] pause timer works
- [ ] `I'm on fire` works
- [ ] local persistence works
- [ ] local data can be deleted
- [ ] daily summary works
- [ ] weekly review works
- [ ] Interest Inbox works
- [ ] Reduce Motion works
- [ ] no critical feature requires internet

---

## 28. Explicit anti-scope-creep list

Do not add before MVP validation:

- chat assistant
- cloud account system
- authentication
- team features
- mobile app
- Apple Watch
- AI scheduling
- automatic calendar editing
- automatic task prioritization
- productivity scoring
- achievements
- streaks
- social features
- browser extension
- screenshot analysis
- content inspection
- automatic project inference
- full audio library
- complex analytics dashboard

When tempted to add one of these, return to the MVP goal.

---

## 29. Build order

Implementation order must follow the milestones in `MILESTONES.md`.

Core rule:

> Do not build reports before trustworthy sessions exist.
> Do not build interventions before Context + Policy can explain why they appear.
> Do not add AI before deterministic behavior feels useful.
