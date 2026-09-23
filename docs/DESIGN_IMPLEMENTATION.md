# Personal Rhythm Assistant — DESIGN_IMPLEMENTATION.md

> Status: active implementation track  
> Version: v0.1  
> Current project state: functional MVP already implemented; real-world behavioral testing underway  
> Purpose: apply the approved visual system to the working product without reopening validated product behavior

---

## 1. Objective

This phase is not a redesign.

The goal is to take the already-working Personal Rhythm Assistant and bring it into visual alignment with the approved Design System v0.1.

Success means:

> The functional app behaves exactly as validated, but now feels like the product designed here: calm, mature, soft, transparent, human, coherent, and visually distinctive.

---

## 2. Frozen behavior

Do not change as part of design implementation:

- Context Engine logic
- Policy Engine logic
- current intervention thresholds
- combined-signal requirement
- 60-minute cooldowns
- 2–3 interventions/day target
- 4/day experimental ceiling
- `I'm on fire` behavior
- privacy restrictions
- local-first architecture
- pause choices
- silence behavior
- daily/weekly review semantics

If design exposes a behavior problem, log it separately.

---

## 3. Canonical visual direction

- neutral ice background
- no photographic nature backgrounds
- soft organic blobs
- pastel color system
- translucent layered surfaces
- restrained glassmorphism
- rounded but mature geometry
- generous whitespace
- soft low-contrast shadows
- calm interface density
- contextual/event cards with different colored borders
- minimal saturated fills
- no visual urgency, scoring, or gamification

The app should feel:

`calm · present · light · mature · gentle · technological without feeling clinical`

It should not feel:

`childish · cartoonish · corporate dashboard · medical · gamified · neon wellness`

---

## 4. Source-of-truth priority

1. approved Design System v0.1
2. latest approved state icons
3. approved high-fidelity My Day direction
4. documented interaction/motion rules
5. earlier boards only as historical reference

Do not use accidental later visual deviations.

---

## 5. Color system

Implement semantic tokens, not hard-coded values.

Families:

- Ice / neutral background
- Deep blue
- Soft blue
- Mint
- Lilac
- Peach
- Aqua
- Warm coral / amber / soft yellow for `I'm on fire`

Suggested tokens:

```css
--color-bg-app;
--color-bg-surface;
--color-bg-glass;
--color-text-primary;
--color-text-secondary;
--color-border-default;

--color-blue;
--color-mint;
--color-lilac;
--color-peach;
--color-aqua;

--color-on-fire-coral;
--color-on-fire-peach;
--color-on-fire-amber;
--color-on-fire-yellow;
```

Rules:
- pastel colors primarily for borders, accents, glows, chips, icons, and light tint
- keep main canvas neutral
- avoid large saturated panels
- never use red as a productivity warning
- `I'm on fire` is warm, not alarming
- color cannot be the only carrier of meaning

---

## 6. Typography

Use the approved clean modern sans-serif direction, based on Inter or the already-approved equivalent.

Hierarchy:

- Display / screen title
- Section heading
- Card title
- Body
- Small / metadata
- Micro label

Rules:
- sentence case
- avoid excessive bold
- use weight and whitespace before increasing size
- keep intervention copy compact
- no oversized motivational-poster treatment

---

## 7. Geometry and surfaces

- large radius for main glass panels
- medium radius for cards
- smaller radius for controls/chips
- avoid pill shapes everywhere
- use thin contextual colored borders
- keep fills mostly neutral/translucent
- use subtle backdrop blur
- low-contrast shadows
- use glass selectively, not on every element

---

## 8. Background system

Main background:

`neutral ice + 2–3 soft blurred blobs`

Blob rules:
- abstract
- low saturation
- aqua/lilac/peach/mint families
- slow motion only where appropriate
- never compete with content
- no photography or leaf/nature imagery
- motion disabled under Reduce Motion

---

## 9. Official icon family

Canonical icon:
- translucent/glass body
- serene closed-eye expression
- calm balance/pause concept
- cool pastel palette for normal state

Required states:
- Normal
- Observing
- Intervention
- I'm on fire
- Silent

Do not use the rejected open-eye or EVE-inspired directions.

### Approved `I'm on fire`

Use:
- same serene closed-eye base
- same silhouette and glass language
- warmer colors only
- peach
- coral
- amber
- soft yellow
- warm glow

Do not use:
- open eyes
- flames
- aggressive expression
- red warning treatment
- gamified energy

---

## 10. My Day

This is the visual reference screen.

Principles:
- neutral ice canvas
- pastel/glass cards
- generous spacing
- calm hierarchy
- colored outlines for event cards
- no busy dashboard density

Approved semantic icon choices:
- `Criar` → test tube / experimentation
- `Recuperar` → recharging battery

Do not use leaf/nature imagery for `Criar`.

My Day should prioritize:
1. current rhythm/context
2. flexible agenda
3. recovery/movement balance
4. next meaningful context
5. optional manual actions

---

## 11. Event cards

Use:
- neutral/translucent fill
- thin colored border
- time
- title
- secondary context
- optional small icon

Avoid full pastel fills for every event.

---

## 12. Intervention

Should feel like the app gently approaching, not an alert.

Use:
- small glass overlay
- neutral background
- official state icon
- concise reason
- 3 energy responses
- clear one-click dismissal

Never use:
- red
- exclamation icon
- blocking modal
- countdown pressure

---

## 13. Pause mode

Pause mode should reduce density.

Use:
- more whitespace
- serene icon
- simple timer
- optional soft blobs
- minimal copy
- very few actions

Do not turn pause into another task screen.

---

## 14. Motion

Canonical language:

`breathe → transform → settle`

Signature:

`pause bars → compress → curve → serene closed eyes → subtle breathing/glow`

Typical behavior:
- ~700 ms transformation
- 3.8–4.5 s ambient breathing cycle
- no continuous blinking

### `I'm on fire` motion

`cool palette → warm peach/coral/amber palette`

~700 ms, with optional slow glow.

No flames.

### Overlay

Entry:
`opacity 0→1 + scale .98→1 + translateY 6→0`, 220–260 ms

Exit:
`opacity 1→0 + translateY 0→4`, ~180–300 ms

No bounce.

---

## 15. Reduce Motion

Mandatory.

When enabled:
- no blob movement
- no rotating state arcs
- no scale breathing
- pause → eyes becomes crossfade
- keep short opacity transitions
- no information depends on animation

---

## 16. Reusable component inventory

At minimum:

- AppShell
- Sidebar / Navigation
- TopBar
- GlassSurface
- Card
- EventCard
- StateIcon
- StatusChip
- PrimaryButton
- SecondaryButton
- GhostButton
- EnergyChoice
- InterventionOverlay
- PauseTimer
- CategoryBadge
- MetricRow
- WeeklyBar
- EmptyState
- SettingsRow
- PrivacyRow

All should use tokens.

---

## 17. Active design milestones

### D0 — Current UI audit

Capture every current screen and classify:

`KEEP · RESTYLE · RESTRUCTURE · REMOVE`

Deliverable: `DESIGN_AUDIT.md`

Do this before broad visual refactoring.

### D1 — Foundations

Implement globally:
- ice background
- colors
- typography
- spacing
- radii
- shadows
- glass
- borders
- focus states
- Reduce Motion foundation

Exit: changing a token updates the app consistently.

### D2 — Icon system

Integrate:
- official app icon
- Normal
- Observing
- Intervention
- approved `I'm on fire`
- Silent
- menu bar/tray treatment

Exit: no obsolete icon remains.

### D3 — App shell

Restyle:
- app background
- blobs
- navigation/sidebar
- title areas
- primary surface hierarchy
- window spacing

Exit: opening the app immediately matches Design System v0.1.

### D4 — My Day

Apply the approved high-fidelity direction.

Includes:
- event borders
- test-tube `Criar`
- battery `Recuperar`
- current rhythm
- daily agenda
- manual actions
- state icon placement

### D5 — Intervention + pause

Refine:

`Intervention → Continue/Break → Pause → Return`

Include signature icon motion.

### D6 — Secondary screens

Apply system to:
- Interest Inbox
- History
- Daily Summary
- Weekly Review
- Settings
- Privacy

Keep density lower than a conventional analytics app.

### D7 — Motion & polish

After visual consistency:
- pause → serene eyes
- state color transitions
- glow
- overlay transitions
- blobs
- layout transitions
- Reduce Motion alternatives

### D8 — Visual QA in real use

Use final visual build for at least one normal workday.

Observe:
- visual fatigue
- blur readability
- excessive pastel
- card density
- state icon legibility
- overlay distraction
- typography scale
- blob distraction
- warm `I'm on fire` clarity

Record findings before redesigning.

---

## 18. Accessibility QA

Validate:
- text contrast over glass
- visible focus states
- keyboard-only operation
- state meaning not dependent only on color
- reduced motion
- no low-contrast pastel body text
- accessible icon labels

Pastel does not justify insufficient contrast.

---

## 19. Do not add during this phase

- new productivity metrics
- new charts
- achievements
- streaks
- new AI features
- mobile companion
- automatic calendar changes
- new behavioral modes

This phase is design convergence, not feature expansion.

---

## 20. Definition of Done

- [ ] Design System v0.1 tokens applied globally
- [ ] neutral ice background consistent
- [ ] approved blobs replace decorative nature imagery
- [ ] typography consistent
- [ ] official app icon integrated
- [ ] all state icons integrated
- [ ] `I'm on fire` uses exact warm serene-eye direction
- [ ] My Day matches approved direction
- [ ] event cards use neutral backgrounds + contextual colored borders
- [ ] Intervention follows approved visual language
- [ ] Pause follows approved visual language
- [ ] Weekly Review follows same system
- [ ] settings/privacy coherent
- [ ] signature motion implemented
- [ ] Reduce Motion implemented
- [ ] obsolete exploratory design removed
- [ ] one full workday completed with final visual build
- [ ] behavioral logic unchanged unless separately approved

---

## 21. Final principle

> The design should make the app feel quieter than the behavior it is trying to correct.

If a visual treatment makes the user more stimulated, more hurried, more scored, or more observed, simplify it.
