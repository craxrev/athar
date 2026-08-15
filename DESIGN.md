---
name: lore
description: A dark, evidence-first record of developer work, where every claim shows how it was established.
colors:
  ground: "#0b0c0f"
  surface-inset: "#0e1014"
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
  git-del: "#e65c63"
  git-del-soft: "rgba(230, 92, 99, 0.14)"
  accent-tint: "#ea439a"
  wash: "rgba(255, 255, 255, 0.022)"
  category-work: "#4c8dff"
  category-work-tint: "#9dc0ff"
  category-research: "#e93d97"
  category-research-tint: "#f79ec9"
  category-personal: "#56c98a"
  category-personal-tint: "#8fdcb1"
  category-freelance: "#f2a93b"
  category-freelance-tint: "#f6c987"
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
  wordmark:
    fontFamily: "{typography.display.fontFamily}"
    fontSize: "19px"
    fontWeight: 640
    lineHeight: 1.3
    letterSpacing: "-0.015em"
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
  control:
    fontFamily: "{typography.display.fontFamily}"
    fontSize: "14px"
    fontWeight: 540
    lineHeight: 1.4
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
  figure:
    fontFamily: "{typography.data.fontFamily}"
    fontSize: "19px"
    fontWeight: 640
    lineHeight: 1.2
    letterSpacing: "-0.01em"
rounded:
  micro: "2px"
  swatch: "3px"
  bar: "4px"
  sm: "6px"
  md: "9px"
  pill: "999px"
  circle: "50%"
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
    backgroundColor: "rgba(255, 255, 255, 0.36)"
    rounded: "{rounded.bar}"
    height: "23px"
  lane-bar-saves:
    backgroundColor: "{colors.uncertain-amber}"
    rounded: "{rounded.bar}"
    height: "23px"
  lane-bar-selected:
    backgroundColor: "{colors.accent-magenta}"
    rounded: "{rounded.bar}"
    height: "23px"
  digest-lead:
    textColor: "{colors.text}"
    typography: "{typography.figure}"
  digest-split:
    textColor: "{colors.text}"
    typography: "{typography.meta}"
  digest-census:
    textColor: "{colors.text-dim}"
    typography: "{typography.meta}"
---

# lore — design system

## Overview

lore reads what other systems leave behind and keeps it after they delete it. The
interface exists to make an uneven record legible without smoothing it, so the
governing idea is not an aesthetic but a discipline: **every figure on screen
carries how it was established.**

The discipline has to hold at the top as well as at the leaf. It is easy to grade
a single commit and then present a week as one confident number; for a while this
build did exactly that, annotating every detail while the two surfaces a person
actually lands on — the digest and the lane bars — said nothing about their own
footing. Both now carry it: the bars in their fill, the digest in a split that
adds up.

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

Each reserved hue carries a lightened **tint** for text sitting on its own fill,
the same three-form rule the category hues follow: `accent-tint` exists because
the solid accent on `accent-soft` measures 4.48:1 and misses the 4.5 floor by
0.02. `git-del` was lifted 4% for the same reason — as text on `surface-hover`,
a state a hovered Stream row genuinely reaches, the original measured 4.31:1.

Magenta is the accent **because** red, green and amber were unavailable: git owns
red and green semantically wherever a diff figure appears, and amber had to stay
free for the honesty markers the product depends on. Any future accent change must
respect the same exclusion.

Five ground steps carry depth, and the ramp runs in both directions: `ground` for
the window, `surface-inset` for a panel opened *inside* a row, `surface` for panes
and resting cards, `surface-raised` for cards inside a pane, `surface-hover` for
pointer feedback. The inset step exists because the ramp otherwise only rose —
expanding a commit in place needed a surface that recedes, and mixing black into
one is how a system starts drifting. Text runs `text` → `text-dim` → `text-faint`; `text-faint` is
the floor and measures 5.95:1 on `ground`, which is where it was set after an
earlier value failed the 4.5:1 requirement.

Category hues (`work`, `research`, `personal`, `freelance`) identify project
groups derived from the filesystem. They never compete with the accent for state,
and they no longer appear on a lane bar — see **The Two Axes Rule** below.

Each hue exists in two forms, defined once as tokens rather than repeated per
component: the **solid** for legend swatches and group headings, and a **fill** at
14–16% for chip backgrounds paired with a lightened **tint** for the text sitting
on it. The tint is not optional — the solid hue is not legible as small text on a
dark ground, which is what the tints exist to solve. Because categories come from
the filesystem the set is open-ended, and an unknown category falls back to
neutral rather than borrowing another category's hue.

**The Two Axes Rule.** Two different questions run through this interface and must
never share a vocabulary. **Attribution** asks who made a change — `witnessed`,
`files match`, `inferred`, `unattributed` — and speaks in the ring icons and the
green/amber pair. **Evidence** asks what kind of record backs a span of time —
`from sessions`, `from commits`, `from saves`, `from records only` — and speaks in
the fill of a lane bar and the split under the digest. A colour or a word borrowed
across the two collapses both. Amber is the one deliberate overlap: it means
uncertainty on either axis, which is why it carries `saves` as well as `inferred`.

## Typography

A workhorse system stack, set solid. There is no display face and no webfont: this
is an Operate surface where legibility at density outranks voice, and the platform
stack renders dense UI text better than anything self-hosted would.

Ramp as actually used: **30 / 21 / 19 / 15.5 / 15 / 14.5 / 14 / 13.5 / 13**. The
`control` step (14px) is the workhorse — rail rows, toolbar buttons, list labels —
and carries the most uses of any step in the build.

**The 13px floor is normative.** `--fs-min: 13px` and `--fs-meta: 13.5px` exist so
that no future surface reintroduces micro-labels. Small type in a light weight is
the specific combination this system rules out.

**The Two Floors Rule.** Weight has two floors, not one, and the difference is
what the text is for. **Interface text never goes below 500** — labels, controls,
rail rows, metadata, figures; that range runs 500–680 and is where the "no skinny
typography" commitment bites. **Reading prose sets at 400**, and only in the
conversation reader and the markdown it renders, at 15px on a 1.62 line-height
across a 78ch measure. The brand constraint was aimed at hairline micro-labels,
not at body copy, where 500 across a two-hundred-message transcript reads heavier
than a long sitting wants. A 400 weight anywhere outside a reading surface is a
defect.

Data is set in the mono stack with `tabular-nums` at the `data` token: times,
durations, token counts, commit shas, line counts, file paths. This is measurement,
not decoration — prose never takes the mono face.

Durations follow two authored rules: a span under a minute prints `<1m`, never
`0m`, because a block holding a single record still happened; and a time range
crossing midnight prints its dates, because clock times alone read as a
contradiction against a multi-day span.

## Layout

Three panes in a row flexbox, sized by the panes themselves rather than by a
parent track list: the rail is a fixed `244px`, the detail pane a fixed `372px`,
both `flex: none`, and the centre takes `flex: 1` with `min-width: 0` so it
absorbs every remaining pixel. The middle pane never collapses; the outer two do,
by keyboard (`⌘B`, `⇧⌘B`) and automatically by width. Reach for a grid only if a
pane ever needs to be resizable — the widths are owned by the panes today, which
is why a hidden pane leaves no reserved track behind.

| Threshold | Behaviour |
|---|---|
| ≤ 1120px | The outer panes become **mutually exclusive**: rail or detail, never both. Detail is the answer to a selection, the rail is how you ask the next question, and both stay one keystroke away |
| any width, rail closed | Toolbar pads left by `--traffic-inset-wide` to clear the window controls |

There is one threshold, not two, and it is **observed state rather than a media
query**. A CSS rule hiding a pane while its `open` flag stayed true meant that
between 880 and 1120px — a band that includes the window's own configured minimum
— selecting a block lit a bar and showed nothing, and the toggle did nothing
visible. Rendering follows state and state follows `matchMedia`, so the two cannot
disagree. Nothing about pane visibility belongs in a stylesheet.

The window uses an overlaid titlebar with hidden title; every pane reserves a
`--titlebar` (30px) drag strip at its top, and the reader's bar indents 62px to
clear the traffic lights.

Rhythm: tight inside a group, generous between groups, and more space above a
heading than below it. Lane rows are 40px with 23px bars; the lane gutter is
`clamp(112px, 16vw, 172px)` so a narrow window gives its width to the time axis
rather than to project names.

The spacing scale in the frontmatter is a **stated target, not an implemented
one**, and deliberately has no tokens. The build uses fourteen distinct values
with no dominant ramp — `10px`, `8px`, `6px`, `9px` and `12px` lead, and none of
them is on the declared six steps. A variable per value would be a dictionary,
not a system, so spacing stays literal until someone decides to move the build
onto a scale rather than describe one it does not follow. Treat the six steps as
binding for new work and as aspiration for old.

Radius went the other way, because it *was* a real system with missing parts: all
seven documented steps are now tokens (`--radius`, `--radius-sm`, `--radius-bar`,
`--radius-swatch`, `--radius-mark`, `--radius-pill`, `--radius-circle`) and no
`border-radius` literal remains in the build. Two structural constants are named
for the same reason: `--traffic-inset` and `--traffic-inset-wide` are where macOS
puts its window controls, measured for two bar heights, and they move together.

Reading measure is capped at 78ch in the conversation reader, centred, regardless
of window width.

## Elevation & Depth

**Structural, not ambient.** Two devices, used for different jobs:

- **Real window vibrancy** (`NSVisualEffectView` via Tauri's `underWindowBackground`)
  on the scope rail *only*. The rail paints almost nothing of its own —
  `rgba(255,255,255,0.022)` — and lets the material through. Dense content never
  sits on a live background.
- **Shadow with offset and blur** on genuinely raised surfaces: detail cards, the
  selected block, the selected lane bar. There is exactly one step, `lift-1`, and
  that is the whole vocabulary. A second step was defined for a surface floating
  above a pane; nothing in this product floats — the reader and settings *replace*
  the timeline rather than hovering over it — so it was deleted rather than left
  standing as a claim the system does not keep. Add a second step when something
  genuinely needs one, not in advance.

1px dividers at 7% white carry structure everywhere else. A raised surface changes
its shadow, not just its border; a surface that only changes its border is not
raised.

Glass is a material with one job here. Blur applied for decoration, or vibrancy
extended to content panes, breaks the rule that separates the rail from everything
else.

## Shapes

Radii: `9px` for panes, block cards and control groups; `6px` for controls and
inner cards; `4px` for lane bars; `3px` for category swatches; `2px` for marks and
commit ticks; `999px` for chips and scrollbars; `50%` for the status dot. Category
swatches are 9px squares at 3px radius — square-ish, so they read as legend keys
rather than bullets, which is why they do not take the pill.

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
- **Digest** runs two tiers, because a flat strip of six equal figures led with
  none of them. The time tier carries elapsed in the `figure` token (19px mono) and
  across-projects a step below it (15px), on purpose: elapsed is wall clock with
  overlaps counted once, across-projects is the sum and may exceed the range. The
  evidence split sits directly beneath across-projects and the census sits below
  both, its values dropped to `text-dim` so supporting counts never read louder
  than the figure they support. Every metric stays an atomic nowrap unit, so a
  wrap lands between metrics and never inside a phrase. The digest does not render
  below one block in range — `duration(0)` floors to `<1m`, and printing that over
  an empty state claims work that did not happen.
- **The evidence split** divides across-projects, never elapsed. Elapsed merges
  overlapping blocks, so splitting it would total more than the whole;
  across-projects is a plain sum, every block carries exactly one class, and the
  parts therefore add up exactly and print with no caveat. A range holding one
  class says `all from sessions` rather than restating a figure it just gave.
- **Lane bars** carry evidence in the fill and nothing else. See below.
- **Empty and partial states** name what they hold rather than rendering blank: a
  block with nothing itemised states its record count; a continued session states
  where it began and that its figures count once.
- **Motion**: one authored moment per surface. Lane bars are revealed from their
  own start along the axis, staggered down the lanes; the reader arrives with a
  260ms settle from an already-visible default. Both on
  `cubic-bezier(0.16, 1, 0.3, 1)`, both disabled under `prefers-reduced-motion`.
  The lane reveal animates `clip-path`, not `scaleX`: scaling squashed the end
  marks of a saves-only bar into slivers and stretched the hatch pitch, distorting
  precisely the two treatments that carry the most meaning. An entrance that
  deforms its own content is not an entrance. Every animation *and* every state
  transition is covered by a `prefers-reduced-motion` guard, per component rather
  than by one global kill — a blanket reset would take the state feedback with it.
- **Motion is chosen by purpose, and every kind is a token.** `--motion-state`
  (120ms ease-out) is the response to a pointer or a toggle. `--motion-live`
  (1.6s ease-in-out) is the one kind that repeats, and it is allowed exactly where
  something is happening right now and will stop on its own: the footer dot while
  a collector runs, in magenta because live state is magenta. A repeating
  animation anywhere else is a defect, and this one is disabled under
  `prefers-reduced-motion` like the rest.

### Evidence class

The system's other signature, and the reason a lane bar gives up its category
hue. A block's start and end are the timestamps of its first and last record, so
*what its width means* changes with what those records are. Drawn identically, a
three-hour conversation and two file saves make the same claim and only one of
them has earned it.

Four treatments, one descending scale of how much of the span is actually known.
One ink drives all four, which is what lets selection recolour a bar without
flattening what it is:

| Class | Treatment | The claim |
|---|---|---|
| `from sessions` | Solid fill, full height | A conversation brackets the whole span, so the whole span is drawn. |
| `from commits` | 45° hatch over a dim bed | Exact at the commits, inferred between them by the idle-gap rule. |
| `from saves` | Amber end caps and a connector | Two mtimes and nothing known between. The ends are marked because the ends are what the archive has. |
| `from records only` | A broken neutral line, no caps | The span is real; nothing in it is describable, so it claims no ends. |

Strongest evidence present wins, and a session counts whether or not its
transcript survived — prompt timestamps are exact, so the span is evidenced even
where the content is gone. That absence is the *attribution* axis, and
`prompts only` already carries it.

**The Ink Rule.** Selection takes the ink, never the shape. A selected saves-only
block is still visibly a pair of end marks in magenta, not a solid claim. Any
state that flattens a treatment into a rectangle has destroyed the only thing the
treatment was there to say.

**The Legible Mark Rule.** Every mark that carries a class clears 3:1 against
`ground` on its own — solid 3.28, hatch strokes 4.22, amber caps 9.79, connector
3.34, selected connector 3.15. A hatch *bed* is exempt because its strokes carry
identity, but a connector is not: without it, two end marks are two unrelated
dots. The category fills this replaced measured 1.60–2.25 and all failed.

Below 14px of rendered width the treatments stop being treatments — a hatch reads
as noise, a pair of end marks touch — so a narrow bar keeps its class colour and
gives up its texture. The lane legend names only the classes the range actually
holds, and does not render below two: a key teaching a code the view is not using
is its own puzzle. The class also rides the bar's `title`, which is reused
verbatim as its `aria-label`, so the one channel the shape cannot reach gets it.

## Do's and Don'ts

**Do**

- Show how a claim was established, next to the claim.
- Keep magenta for selection and live state; keep amber for uncertainty.
- Set data in the mono token with tabular numerals.
- Say what a surface does hold when it cannot show what was expected.
- Give a raised surface a shadow with offset and blur.
- Let a mark that carries meaning clear 3:1 against its ground on its own.
- Split a figure only where the parts add up. Elapsed does not; across-projects
  does.

**Don't**

- Set interface type below 13px or below weight 500. Reading prose is the one
  exception and sets at 400.
- Use the accent as a glow, a gradient, or a decorative fill.
- Extend vibrancy beyond the scope rail, or add blur for atmosphere.
- Render a calendar grid or a contribution heatmap as a primary view.
- Print a figure lore cannot support — no `0m` for a real block, no clock range
  without dates across days, no total where the data only supports a floor, no
  digest over a range holding nothing.
- Introduce a second accent, or let a category hue carry state.
- Put a category hue back on a lane bar. The group heading above it and the rail
  row that filtered to it already say it; the fill is spent on evidence now.
- Borrow a word or a colour across the attribution and evidence axes.
- Animate a bar with a transform that deforms the marks inside it.
