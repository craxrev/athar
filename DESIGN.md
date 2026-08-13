---
name: lore
description: A dark, evidence-first record of developer work, where every claim shows how it was established.
colors:
  ground: "#0b0c0f"
  surface: "#12141a"
  surface-raised: "#191c24"
  surface-hover: "#1f2330"
  line: "rgba(255, 255, 255, 0.07)"
  line-strong: "rgba(255, 255, 255, 0.14)"
  text: "#edeef2"
  text-dim: "#a2a8b6"
  text-faint: "#878e9d"
  accent-magenta: "#e93d97"
  accent-soft: "rgba(233, 61, 151, 0.16)"
  accent-edge: "rgba(233, 61, 151, 0.42)"
  on-accent: "#14040c"
  uncertain-amber: "#f2a93b"
  uncertain-amber-soft: "rgba(242, 169, 59, 0.14)"
  git-add: "#56c98a"
  git-del: "#e5555c"
  category-work: "#4c8dff"
  category-research: "#e93d97"
  category-personal: "#56c98a"
  category-freelance: "#f2a93b"
typography:
  display:
    fontFamily: "-apple-system, BlinkMacSystemFont, Segoe UI, system-ui, sans-serif"
    fontSize: "30px"
    fontWeight: 680
    lineHeight: 1.2
    letterSpacing: "-0.024em"
  title:
    fontFamily: "{typography.display.fontFamily}"
    fontSize: "21px"
    fontWeight: 660
    lineHeight: 1.3
    letterSpacing: "-0.02em"
  subtitle:
    fontFamily: "{typography.display.fontFamily}"
    fontSize: "15.5px"
    fontWeight: 620
    lineHeight: 1.4
    letterSpacing: "-0.01em"
  body:
    fontFamily: "{typography.display.fontFamily}"
    fontSize: "15px"
    fontWeight: 400
    lineHeight: 1.5
    letterSpacing: "normal"
  label:
    fontFamily: "{typography.display.fontFamily}"
    fontSize: "14.5px"
    fontWeight: 545
    lineHeight: 1.35
    letterSpacing: "normal"
  meta:
    fontFamily: "{typography.display.fontFamily}"
    fontSize: "13.5px"
    fontWeight: 500
    lineHeight: 1.45
    letterSpacing: "normal"
  data:
    fontFamily: "ui-monospace, SF Mono, JetBrains Mono, Menlo, monospace"
    fontSize: "13px"
    fontWeight: 560
    lineHeight: 1.4
    letterSpacing: "normal"
rounded:
  bar: "4px"
  sm: "6px"
  md: "9px"
  pill: "999px"
spacing:
  hairline: "2px"
  xs: "4px"
  sm: "7px"
  md: "11px"
  lg: "18px"
  xl: "26px"
components:
  rail-row:
    backgroundColor: "transparent"
    textColor: "{colors.text-dim}"
    rounded: "{rounded.sm}"
    padding: "7px 8px"
    typography: "{typography.label}"
  rail-row-hover:
    backgroundColor: "{colors.surface-hover}"
    textColor: "{colors.text}"
  rail-row-selected:
    backgroundColor: "{colors.accent-soft}"
    textColor: "{colors.text}"
  view-toggle:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.text-dim}"
    rounded: "{rounded.sm}"
    padding: "5px 11px"
  view-toggle-selected:
    backgroundColor: "{colors.accent-soft}"
    textColor: "{colors.text}"
  block-card:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.text}"
    rounded: "{rounded.md}"
    padding: "11px 14px"
  block-card-selected:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.text}"
  detail-card:
    backgroundColor: "{colors.surface-raised}"
    textColor: "{colors.text}"
    rounded: "{rounded.sm}"
    padding: "11px 12px"
  filter-field:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.text}"
    rounded: "{rounded.sm}"
    padding: "0 10px"
    height: "30px"
  category-chip:
    backgroundColor: "{colors.uncertain-amber-soft}"
    textColor: "{colors.text-dim}"
    rounded: "{rounded.pill}"
    padding: "2px 7px"
    typography: "{typography.meta}"
  lane-bar:
    backgroundColor: "rgba(255, 255, 255, 0.17)"
    rounded: "{rounded.bar}"
    height: "23px"
  lane-bar-selected:
    backgroundColor: "{colors.accent-magenta}"
    rounded: "{rounded.bar}"
    height: "23px"
---

# lore — design system

## Overview

lore reads what other systems leave behind and keeps it after they delete it. The
interface exists to make an uneven record legible without smoothing it, so the
governing idea is not an aesthetic but a discipline: **every figure on screen
carries how it was established.**

The system is dark only, and by decision rather than by category habit — this is
read at a desk, on the machine that produced the work, usually late. Its restraint
is the restraint of an instrument panel: one accent, no ornament, dense
information set at sizes that stay comfortable for a long look.

Anti-reference: the calendar grid and the contribution heatmap. Time appears as
lanes of real projects along a real axis, never as dates in boxes. The build
deliberately executes the conventional desktop-tool form at full craft rather than
dressing the product in a metaphor.

## Colors

**Strategy: restrained.** Dark neutrals carry every surface; one accent carries
state. Colour is never decorative here, because three of the useful hues are
already spoken for by meaning:

| Role | Token | Reserved for |
|---|---|---|
| Selection, live state | `accent-magenta` | The current selection and anything actionable. Flat fill only. |
| Uncertainty | `uncertain-amber` | Inferred attribution, coverage gaps, unreachable commits, coarse timestamps |
| Insertions | `git-add` | Added lines, and the "witnessed" confidence tier |
| Deletions | `git-del` | Removed lines, failed tool calls |

Magenta is the accent **because** red, green and amber were unavailable: git owns
red and green semantically wherever a diff figure appears, and amber had to stay
free for the honesty markers the product depends on. Any future accent change must
respect the same exclusion.

Four ground steps carry depth: `ground` for the window, `surface` for panes and
resting cards, `surface-raised` for cards inside a pane, `surface-hover` for
pointer feedback. Text runs `text` → `text-dim` → `text-faint`; `text-faint` is
the floor and sits at ~5.2:1 on `ground`, which is where it was set after
measuring an earlier value that failed the 4.5:1 requirement.

Category hues (`work`, `research`, `personal`, `freelance`) identify project
groups derived from the filesystem. They tint lane bars and chips; they never
compete with the accent for state.

## Typography

A workhorse system stack, set solid. There is no display face and no webfont: this
is an Operate surface where legibility at density outranks voice, and the platform
stack renders dense UI text better than anything self-hosted would.

Ramp as actually used: **30 / 21 / 19 / 15.5 / 15 / 14.5 / 13.5 / 13**. Weights run
500–680; nothing is set lighter than 500.

**The 13px floor is normative.** `--fs-min: 13px` and `--fs-meta: 13.5px` exist so
that no future surface reintroduces micro-labels. Small type in a light weight is
the specific combination this system rules out.

Data is set in the mono stack with `tabular-nums` at the `data` token: times,
durations, token counts, commit shas, line counts, file paths. This is measurement,
not decoration — prose never takes the mono face.

Durations follow two authored rules: a span under a minute prints `<1m`, never
`0m`, because a block holding a single record still happened; and a time range
crossing midnight prints its dates, because clock times alone read as a
contradiction against a multi-day span.

## Layout

Three panes on a CSS grid: `244px | minmax(0, 1fr) | 372px`. The middle pane never
collapses; the outer two do, by keyboard (`⌘B`, `⇧⌘B`) and automatically by width.

| Breakpoint | Behaviour |
|---|---|
| ≤ 1120px | Detail pane folds away — below this it left the timeline too narrow to read |
| ≤ 880px | Scope rail folds away; toolbar pads left for the traffic lights |

The window uses an overlaid titlebar with hidden title; every pane reserves a
`--titlebar` (30px) drag strip at its top, and the reader's bar indents 62px to
clear the traffic lights.

Rhythm: tight inside a group, generous between groups, and more space above a
heading than below it. Lane rows are 40px with 23px bars; the lane gutter is
`clamp(112px, 16vw, 172px)` so a narrow window gives its width to the time axis
rather than to project names.

Reading measure is capped at 78ch in the conversation reader, centred, regardless
of window width.

## Elevation & Depth

**Structural, not ambient.** Two devices, used for different jobs:

- **Real window vibrancy** (`NSVisualEffectView` via Tauri's `underWindowBackground`)
  on the scope rail *only*. The rail paints almost nothing of its own —
  `rgba(255,255,255,0.022)` — and lets the material through. Dense content never
  sits on a live background.
- **Shadow with offset and blur** (`lift-1`, `lift-2`) on genuinely raised
  surfaces: detail cards, the selected block, the selected lane bar.

1px dividers at 7% white carry structure everywhere else. A raised surface changes
its shadow, not just its border; a surface that only changes its border is not
raised.

Glass is a material with one job here. Blur applied for decoration, or vibrancy
extended to content panes, breaks the rule that separates the rail from everything
else.

## Shapes

Radii: `9px` for panes and block cards, `6px` for controls and inner cards, `4px`
for lane bars, `999px` for chips and status dots. Category swatches are 9px squares
at 3px radius — square-ish, so they read as legend keys rather than bullets.

Icons are a single drawn set on a 20px grid: 1.75 stroke, round caps and joins,
`currentColor`. Two of them encode meaning rather than objects — a closed ring for
a witnessed claim, the same ring broken for an inferred one. No glyph, emoji or
character stands in for an icon anywhere.

## Components

- **Selection** is a flat accent fill at 16% alpha with a 42% alpha inset edge, and
  the icon inside turns accent. Never a glow, never a gradient.
- **Confidence labels** are the system's signature: `witnessed` in `git-add`,
  `files match` in `text-dim`, `inferred` and `unattributed` in
  `uncertain-amber`, each with the ring icon that matches its certainty. The detail
  pane spells the reasoning out in a sentence.
- **Honesty flags** are amber pills: `prompts only` where the source deleted a
  transcript, `only in lore` where git will collect a commit lore has kept.
- **Digest line** sets figures in the data token inline, each metric an atomic
  nowrap unit so a wrap lands between metrics and never inside a phrase. It carries
  two time figures on purpose — elapsed (wall clock, overlaps counted once) and
  across-projects (the sum, which may exceed the range).
- **Lane bars** tint by category, carry up to four commit ticks, and hide those
  ticks below 1.4% of track width so the marker never eats the thing it annotates.
- **Empty and partial states** name what they hold rather than rendering blank: a
  block with nothing itemised states its record count; a continued session states
  where it began and that its figures count once.
- **Motion**: one authored moment per surface. Lane bars grow from their own start
  along the axis, staggered down the lanes; the reader arrives with a 260ms settle
  from an already-visible default. Both on `cubic-bezier(0.16, 1, 0.3, 1)`, both
  disabled under `prefers-reduced-motion`.

## Do's and Don'ts

**Do**

- Show how a claim was established, next to the claim.
- Keep magenta for selection and live state; keep amber for uncertainty.
- Set data in the mono token with tabular numerals.
- Say what a surface does hold when it cannot show what was expected.
- Give a raised surface a shadow with offset and blur.

**Don't**

- Set type below 13px, or below weight 500.
- Use the accent as a glow, a gradient, or a decorative fill.
- Extend vibrancy beyond the scope rail, or add blur for atmosphere.
- Render a calendar grid or a contribution heatmap as a primary view.
- Print a figure lore cannot support — no `0m` for a real block, no clock range
  without dates across days, no total where the data only supports a floor.
- Introduce a second accent, or let a category hue carry state.
