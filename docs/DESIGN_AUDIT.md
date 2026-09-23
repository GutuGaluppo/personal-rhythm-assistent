# Personal Rhythm Assistant — DESIGN_AUDIT.md

> D0 deliverable per `docs/DESIGN_IMPLEMENTATION.md` §17. Captures every current
> screen/component and classifies it before any broad visual refactoring starts.
>
> Classification:
> - **KEEP** — behavior and structure are right; no visual work needed (or out of scope, e.g. developer-only).
> - **RESTYLE** — structure/markup stays; only tokens, surfaces, colors, type, icons change.
> - **RESTRUCTURE** — the DOM/layout itself needs to change to match the approved direction (new elements, different composition), without changing the underlying behavior it renders.
> - **REMOVE** — nothing in the current app corresponds to this; only relevant if something should be deleted (none found).

Source of truth used, in priority order (per §4): the Design System v0.1 poster
(`personal_rhythm_assistant_design_system.png` / `wide_pastel_ui_design_system_poster_style_guide.png`,
identical content), the state-icon panel within it, the approved My Day mockup
(`personal_rhythm_assistant_ui_poster.png`, panel 1), and `DESIGN_IMPLEMENTATION.md` itself.
`pastel_wellness_productivity_dashboard.png` and the individual icon explorations
in `Approved/` are treated as historical/supporting reference only, per §4 rule 5 —
their category iconography and sidebar item list (Agenda, Planejamento, Relatórios
as separate nav destinations) are **not** adopted where they conflict with the
poster or with §19 ("do not add new charts/metrics during this phase").

---

## 1. Foundations

| Item | File(s) | Classification | Notes |
| --- | --- | --- | --- |
| Color tokens | `src/design-system/tokens/tokens.css` | **RESTYLE** | File header literally says "placeholder values until Design System v0.1 is imported." Needs the 7 palette colors, semantic surface/text/border tokens, and the on-fire warm family from §5. |
| Base reset | `src/design-system/tokens/base.css` | **RESTYLE** | Body background/font need the new tokens; otherwise fine. |
| Typography | tokens + no dedicated type file yet | **RESTRUCTURE** | No type-scale tokens exist today (font sizes are hard-coded per component, e.g. `1.4rem`, `0.85rem`). Needs new tokens for the 6-level hierarchy in §6, then every component switches to them (a RESTYLE once the tokens exist). |
| Spacing / radius | tokens.css | **RESTYLE** | Spacing scale already exists (4/8/16/24) and is close; needs the 4/8/12/16/24/32 scale and a large/medium/small radius split (currently one `--radius-md: 12px` only). |
| Shadows / glass / blur | none | **RESTRUCTURE** | No shadow or blur tokens exist at all; §7's "Card sutil / Card elevado / Painel" surface levels need new tokens and a `GlassSurface`-style primitive that doesn't exist yet. |
| Motion durations | `src/design-system/motion/durations.ts`, tokens.css | **KEEP** | Already matches IMPLEMENTATION.md §23 exactly (micro 190ms ≈160-220, overlay 240ms, transform 700ms, breathe 4400ms). No change needed. |
| Reduce Motion plumbing | tokens.css, `useReduceMotion` | **KEEP** | Already implemented and tested; new motion work must keep using it, not replace it. |

## 2. Background / app shell

| Item | File(s) | Classification | Notes |
| --- | --- | --- | --- |
| Ice background + blobs | — (none today) | **RESTRUCTURE** | Currently a flat `--color-bg` fill. §8 requires a neutral ice canvas with 2-3 soft blurred blobs (aqua/lilac/peach/mint), disabled under Reduce Motion. New component needed (e.g. `BackgroundBlobs`), mounted once in `App.tsx`/`root.tsx`. |
| Nav / shell layout | `src/app/App.tsx`, `App.module.css` | **RESTYLE** | Already a left-column + main grid (`200px 1fr`), which is structurally close to the approved sidebar — no restructuring needed, just glass surface, spacing, active-item treatment, and the app icon/wordmark lockup from the poster's sidebar. Do **not** add the extra nav destinations (Agenda/Planejamento/Relatórios as separate items) shown in the dashboard mockup — those aren't in the current product's page set and adding them is out of scope for this phase (§19). |
| Main window title/icon | `src-tauri/tauri.conf.json`, `src-tauri/icons/*` | **RESTRUCTURE** | Icon set is the Tauri scaffold default, unrelated to the product. Needs regenerating from the approved icon mark (D2). |

## 3. Icon system (D2)

| Item | File(s) | Classification | Notes |
| --- | --- | --- | --- |
| App icon (all sizes/platforms) | `src-tauri/icons/*.png`, `.ico`, `.icns` | **RESTRUCTURE** | Currently the Tauri default icon. Needs full regeneration from the approved glass/serene-eyes mark. |
| Tray/menu-bar state icons | `src-tauri/icons/tray/*.png`, `scripts/make-state-icons.py` | **RESTRUCTURE** | `make-state-icons.py`'s own docstring says these are deliberate placeholders "until the approved Design System icons arrive." They have arrived. Needs Normal/Observing/Intervention/On-fire/Silent variants of the approved mark, monochrome-safe for the menu bar per the poster's "Menu bar · Monocromático" variant. |
| `PauseVisual` (bars → eyes motion) | `src/components/pause/PauseVisual.tsx` | **KEEP** | The morph logic (bars → compress → curve → serene eyes, with a crossfade fallback) already *is* the approved signature motion structurally. It draws in `--color-accent` only, so it inherits the new palette for free once tokens change — no code change needed here, just verify the resulting look against the icon panel once D1 lands. |

## 4. My Day (D4, the visual reference screen)

| Item | File(s) | Classification | Notes |
| --- | --- | --- | --- |
| Page layout / cards | `src/features/my-day/MyDay.tsx`, `.module.css` | **RESTYLE** | Card grid of "Current session / Today / Last real break / Today's plan / Interest suggestion" already matches the approved card-based composition (§10, §11) reasonably well. Needs glass card surfaces, colored event borders, the state icon in place of plain text, generous spacing — no new cards, no donut/line charts (excluded by §19; those only appear in the historical dashboard mockup, not the approved poster's My Day panel or the textual spec). |
| Category naming/icons | `src/components/rhythm/CategoryBars.tsx` (if used), category display in cards | **RESTYLE** | Category set (Create/Learn/Explore/Move/Life/People/Recover/Think/Unknown) is frozen product behavior (do not rename/restructure). Only the *icon* for Create (test tube, not leaf) and Recover (recharging battery, per §10 — note the dashboard mockup shows a lightning bolt for "Recuperar"; the textual spec's battery is authoritative) needs to change. |
| "Take a break" button, quick actions | `MyDay.tsx` header | **RESTYLE** | Becomes a `PrimaryButton`/`SecondaryButton` per the new component tokens; placement/behavior unchanged. |

## 5. Intervention window (D5)

| Item | File(s) | Classification | Notes |
| --- | --- | --- | --- |
| Overlay chrome | `src/features/intervention/InterventionWindow.tsx`, `.module.css` | **RESTYLE** | Structure already matches §12 closely: state icon, headline, question, energy/action choices, one-click dismiss, visible reasons. Needs `GlassSurface` treatment, neutral background, official state icon, and softened button/chip styling — no structural change. |
| Energy/action choice buttons | same file | **RESTYLE** | Map onto the poster's 3-choice row (Bem/Ok/Cansado(a)) styling — pill/chip shape, soft colored fill, not the current plain buttons. |

## 6. Pause mode (D5)

| Item | File(s) | Classification | Notes |
| --- | --- | --- | --- |
| Setup screen | `src/features/pause/PauseWindow.tsx` (`Setup`) | **RESTYLE** | Kind/duration/reason selection already exists; needs glass surface, chip styling for kind + reason pickers (already chip-shaped, just needs tokens), generous whitespace per §13. |
| Running/untimed screen | same file (`Session`) | **RESTYLE** | Timer/elapsed display, prompt, single primary action already match "very few actions, minimal copy." Needs the big circular/soft timer treatment implied by the poster's "Pausa ativa" panel — this is a visual (RESTYLE), not structural, change: the same `formatRemaining`/elapsed values just render inside a rounder, larger glass treatment. |
| Reason chips | same file (`ReasonPicker`) | **RESTYLE** | Already a `StatusChip`-shaped control; needs token colors only. |

## 7. Secondary screens (D6)

| Item | File(s) | Classification | Notes |
| --- | --- | --- | --- |
| Interest Inbox | `src/features/interest-inbox/*` | **RESTYLE** | Simple list/form; needs card/glass/chip tokens, no structural change. |
| Today's plan | `src/features/my-day/DailyPlan.tsx`, `.module.css` | **RESTYLE** | Same treatment as Interest Inbox; it's the newest feature and already uses the same plain-card pattern. |
| History (daily/weekly tabs) | `src/features/history/*` | **RESTYLE** | Tab control needs the poster's segmented "Dia/Semana/Mês"-style control instead of underline tabs. |
| Daily Summary / Weekly Review | `src/features/reports/*` | **RESTYLE** | Card/metric-row treatment only; keep "no productivity score" copy exactly as-is (frozen behavior). |
| Settings / Privacy | `src/features/settings/*` | **RESTYLE** | `SettingsRow`/`PrivacyRow`/toggle components from §16's inventory don't exist as named primitives yet; introducing them is a RESTYLE of existing rows into a shared component, not a behavior change. |
| Context Debug | `src/features/debug/ContextDebug.tsx` | **KEEP** | Developer-only, excluded from production nav (`pagesFor(isDevelopment)`); no design requirement applies. |

## 8. Net-new primitives required (§16 inventory)

None of these exist as reusable components today; each current usage listed is an
inline/ad-hoc version to be replaced by the shared primitive during D1/D3:

| Primitive | Currently implemented (ad hoc) as... |
| --- | --- |
| `GlassSurface` | plain `.card`/`.page` backgrounds in every module |
| `Card` | `src/components/ui/Card.tsx` — **KEEP the component, RESTYLE its CSS** |
| `EventCard` | inline markup in `MyDay.tsx`'s session/break rows |
| `StateIcon` | tray PNGs (Rust side) + no frontend equivalent yet |
| `StatusChip` | inline `<button>` chip styling repeated in `PauseWindow.tsx`, `Settings.tsx` |
| `PrimaryButton` / `SecondaryButton` / `GhostButton` | plain `<button>` + module CSS classes (`.primary`, `.quiet`) repeated per file |
| `EnergyChoice` | inline buttons in `InterventionWindow.tsx` |
| `PauseTimer` | inline `<p role="timer">` in `PauseWindow.tsx` |
| `EmptyState` | inline `<p className={styles.muted}>` scattered across list views |
| `SettingsRow` / `PrivacyRow` | inline rows in `Settings.tsx` and related files |

Building these as shared components (rather than restyling each file's bespoke
markup independently) is recommended so D1's token change and D3-D6's rollout
stay consistent — this becomes part of D1/D3 scope, not a new milestone.

## 9. Summary

- Nothing is classified **REMOVE**: no current screen or component falls outside the approved direction: it all just needs restyling (or, for background/icons/type-scale/shadow tokens, building for the first time).
- **RESTRUCTURE** items are all additive (background blobs, icon regeneration, shadow/type/glass tokens, shared component primitives) — none require deleting or reworking existing product behavior, consistent with §2's frozen-behavior list.
- No item in the approved references requires reopening Context Engine, Policy Engine, thresholds, cooldowns, or privacy behavior.
- Recommended order matches `DESIGN_IMPLEMENTATION.md` §17 as written: **D1 (foundations) → D2 (icons) → D3 (shell) → D4 (My Day) → D5 (intervention/pause) → D6 (secondary screens) → D7 (motion polish) → D8 (real-use QA)**.
