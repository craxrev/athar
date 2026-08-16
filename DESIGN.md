---
name: lore
description: A dark, evidence-first record of developer work, where every claim shows how it was established.
colors:
  ground: "#0b0c0f"
  surface-inset: "#0e1014"
  surface: "#12141a"
  surface-raised: "#191c24"
  surface-hover: "#1f2330"
  well: "rgba(255, 255, 255, 0.03)"
  mark-cut: "rgba(0, 0, 0, 0.42)"
  line: "rgba(255, 255, 255, 0.07)"
  line-strong: "rgba(255, 255, 255, 0.14)"
  text: "#edeef2"
  text-dim: "#a2a8b6"
  text-faint: "#878e9d"
  accent-magenta: "#e93d97"
  accent-soft: "rgba(233, 61, 151, 0.16)"
  accent-edge: "rgba(233, 61, 151, 0.42)"
  uncertain-amber: "#f2a93b"
  uncertain-amber-soft: "rgba(242, 169, 59, 0.14)"
  git-add: "#56c98a"
  git-del: "#e65c63"
  git-del-soft: "rgba(230, 92, 99, 0.14)"
  accent-tint: "#ea439a"
  wash: "rgba(255, 255, 255, 0.022)"
  fill-subtle: "rgba(255, 255, 255, 0.06)"
  scrim: "rgba(0, 0, 0, 0.45)"
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
  timeline-mark:
    backgroundColor: "{colors.text-faint}"
    rounded: "{rounded.micro}"
    height: "20px"
  timeline-mark-saves:
    backgroundColor: "{colors.uncertain-amber}"
    rounded: "{rounded.micro}"
    height: "20px"
  timeline-mark-selected:
    backgroundColor: "{colors.accent-magenta}"
    rounded: "{rounded.micro}"
    height: "20px"
  day-tile:
    backgroundColor: "{colors.surface}"
    rounded: "{rounded.sm}"
    padding: "6px 7px 7px"
    height: "82px"
  day-cell:
    backgroundColor: "{colors.text-faint}"
    rounded: "{rounded.swatch}"
    size: "10px"
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
actually lands on — the digest and the timeline — said nothing about their own
footing. Both now carry it: the marks in their texture, the digest in a split
that adds up.

The system is dark only, and by decision rather than by category habit — this is
read at a desk, on the machine that produced the work, usually late. Its restraint
is the restraint of an instrument panel: one accent, no ornament, dense
information set at sizes that stay comfortable for a long look.

**The grain ladder.** The timeline's unit is the **day**, and what survives of a
day is set by the range it is read at. The rail's scopes were always named for
the unit rather than the window — "By day", "By week" — and the view now honours
what they hand it:

| Scope | A day is | and it still carries |
|---|---|---|
| By day | an **hour** | exact times, one row per project |
| By week | a **row** | all twenty-four hours, resolved |
| By month | a **tile** | its date, its hours, what was worked on |
| All time | a **cell** | its position and its ink |

**Which rung a range lands on is computed once, in `src/lib/grain.ts`, and the
timeline and the keyboard both read it.** The rail selects a *unit* and the range
gives a *length*, and the two do not always agree — "all time" on a two-week-old
archive is a week of days, not seven years of cells — so the rung has to be
derived rather than read off the scope. It has to be derived in one place for a
sharper reason: the marks the reader can *see* and the marks the reader can
*reach* are the same set, and two copies of the rule is exactly how they stopped
being. Under a second copy, `j` at month grain selected a block that no rung had
drawn: the pane described it, every tile on screen receded around it, and nothing
lit up.

**A rung draws the days of the range, never the days of a calendar unit.** The
tile rung is reached at `all time` whenever the archive is younger than about six
weeks, and a grid built from one month's first-of-month drew that month and
stopped — on a 20 July to 16 August archive, sixteen of twenty-eight days in
range never rendered while nineteen days from before the archive existed rendered
as tiles reading "nothing archived". It now emits a section per month the range
touches, headed only when there is more than one, and a day inside a drawn month
but outside the range is a **pad, never an empty tile**: the archive was never
asked about it, and "nothing archived" is a claim like any other.

Each rung drops exactly one channel, and the drop happens where the previous
rung stops fitting. **Category grouping lives on the day rung and nowhere else.**
Rows are projects only there, so that is the only rung where a heading can name a
class and sum it; above it a category can be a hue and nothing more, and the
per-class totals move to the rail. Dropping the heading everywhere was the
ladder's one silent subtraction, and it is restored where it means something
rather than everywhere it used to sit.

**The ladder is also the target-size answer.** A year of days
at a pointer-sized 24px would need 1,272px of width for the columns alone, so the
cell rung is deliberately below WCAG 2.5.8's minimum and relies on that criterion's
equivalent-control exception: the same day is reachable at 22px in the months
sheet and at 110×82px as a tile, one and two clicks in. A rung that shrinks the
target must hand the reader a larger one for the same job. Thirty days as rows is twenty-six pixels each and a wasted
axis; as tiles it is a page you can read. The tile is the last rung where a day
can still say a name.

This replaces the earlier rule that time appears only as lanes of real projects
along one shared axis. That rule was written when every range drew the same way,
and it is what made all time unreadable: over seven years the archive floors
twenty-two of every twenty-three blocks at the minimum bar width, so fifteen
hours of work rendered as a row of identical dots. Project lanes survive at day
grain, which is the range where blocks are wide enough to compare side by side.

**The calendar anti-reference is revoked, deliberately and with the measurement
that revoked it.** A grid of dates was banned as a metaphor standing in for the
work. At all time it is not a metaphor: it is the only arrangement that shows
2,697 days at once and still resolves one of them. What the ban was protecting
against — dates in boxes standing in for evidence — is answered by the cells
carrying real hours and real class, not by refusing the shape. The build still
executes the conventional desktop-tool form at full craft and still refuses to
dress the product in a metaphor.

## Colors

**Strategy: restrained.** Dark neutrals carry every surface; one accent carries
state. Colour is never decorative here, because three of the useful hues are
already spoken for by meaning:

| Role | Token | Reserved for |
|---|---|---|
| Selection, live state | `accent-magenta` | The current selection, and the one thing happening right now. Flat fill only. |
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

**"Anything actionable" was too wide a door, and it was quietly walked through
three times.** The session icon in Stream, the "You" label on every user turn in
the reader, and the "wrote" badge on a touched file had all taken the accent —
each defensible alone, and collectively fatal to the thing the accent is for. A
day in Stream holds twenty blocks and a block holds several sessions, so an
accented row type puts dozens of magenta marks on screen competing with the one
accent fill that means *this is the block you chose*; a two-hundred-turn
transcript spends it a hundred times on neither selection nor live state. All
three were taken back rather than written into the rule: the row types are told
apart by their drawn icons, "You" leads on the ink ramp, and `wrote` takes
`git-add`, because what a session did to a file is git's vocabulary and not the
accent's. **A control being clickable does not earn the accent. Being chosen
does.**

Five ground steps carry depth, and the ramp runs in both directions: `ground` for
the window, `surface-inset` for a panel opened *inside* a row, `surface` for panes
and resting cards, `surface-raised` for cards inside a pane, `surface-hover` for
pointer feedback. The inset step exists because the ramp otherwise only rose —
expanding a commit in place needed a surface that recedes, and mixing black into
one is how a system starts drifting. Text runs `text` → `text-dim` → `text-faint`; `text-faint` is
the floor and measures 5.95:1 on `ground`, which is where it was set after an
earlier value failed the 4.5:1 requirement.

**Category hues are derived, not declared.** Categories come from the scanned
roots, which are configuration — a person names one whatever they like and adds a
twelfth whenever they like. Four hues named in CSS could only ever colour four,
and did it by matching literal strings in thirty-two selectors, so a rename or a
capital letter dropped a category to grey with nothing said. `src/lib/palette.ts`
hashes the normalised name into a palette of **ten** and hands back a solid and a
tint; components set `--cat` and `--cat-tint` and the CSS reads those.

The palette excludes every hue this system already spends meaning on: magenta is
selection and live state, amber is uncertainty, git owns red and green. The old
four broke that three times over — `research` *was* the accent's exact value,
`personal` was `git-add`'s, `freelance` was amber's — which is why a selected
rose mark was indistinguishable from an unselected one. Each entry is generated
at one OKLCH lightness and chroma so no category is louder than another, and each
solid clears 4:1 on ground even at the density floor.

**The palette can be re-dealt.** Sorting gives one arrangement, and a pairing the
reader dislikes would otherwise need a root renamed to escape. A seed permutes
the palette, and Settings carries a **Shuffle colours** action beside the roots
list that re-deals it — the same categories, a different set of colours. The seed
is the only thing kept, alongside the view and range the window reopens on; no
colour is ever stored against a category. Shuffling changes which colour lands
where, never how many there are.

Ten hues for an unbounded set of names means an eleventh category shares with a
first. That is stated rather than designed away: two categories on one hue are
told apart by the name beside them, never by the mark alone. An empty category is
given a hue at all — an uncategorised path is the absence of a category, not one
of them, and neutral is the honest answer. That is not only a point of principle:
it is routinely the largest group, so colouring it spent the loudest mark on
screen on the least meaning and cost a real category a slot. `text` would be
worse than a hue, being the brightest ink the system has.

**The Density Floor Rule.** A cell's hours ride its **hue**, never its alpha
alone. Ramping opacity from near-nothing was the obvious way to draw density and
it put most days between 1.3:1 and 2.3:1 against ground — under the 3:1 that the
Legible Mark Rule demands of anything carrying a class, on the very marks the
widest ranges are made of. The floor is `0.72` alpha, set by the worst case:
the palette's least luminous hue reaches 3:1 just above `0.70`. Density then rides a mix toward the
category's own **tint**, which is what the tints were minted for. A quiet day
measures 3.14:1 and a heavy one 7.26:1 — a full step apart, both legal.

Two utility neutrals carry the parts of a mark that are not the mark: `well` is
the empty half of a mark's own container — the track behind a span, an unheld
cell — and `mark-cut` is the dark that is *subtracted* from a mark to texture it,
in the commits hatch and the records-only break. Both existed as four
hand-copied alphas before they had names.

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
the split under the digest and in the **texture** of a mark on the timeline. A
colour or a word borrowed across the two collapses both. Amber is the one
deliberate overlap: it means uncertainty on either axis, which is why it carries
`saves` as well as `inferred`.

**Hue and texture are separate channels, and a mark may carry both.** Evidence
used to own a lane bar's fill outright, because a lane was a project and its
category was already stated by the heading above it. Under the grain ladder a row
is a day and holds several projects at once, so the mark has to say whose work it
was: **hue carries project class, texture carries evidence.** Solid for
`sessions`, a 45° hatch for `commits`, amber end caps for `saves`, a broken line
for `records only` — the same four treatments, moved off the fill and onto the
surface. Two questions, two channels, still no shared vocabulary. This is the
narrow exception to the rule below about keeping category hue off a timeline
mark, and it is narrow on purpose: hue may say *which project*, never *what
state*. State stays magenta.

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

## Copy

Plain and short. The reader is one developer looking at their own work on their
own machine, so domain words are free — commit, mtime, reflog, block, session,
rebuild all land without help. What does not land is lore explaining itself.

**One rule decides every string: keep what qualifies a figure, cut what explains
the build.** The copy had drifted into this document's own register — compound
sentences, em-dash asides, a justification after every statement — which reads
well on a page and badly in a pane you are trying to get an answer out of. An
error state offering an architecture lesson before its two buttons is the shape
to watch for.

- **A heading that already states the case gets no restating sentence.** If the
  body has nothing new, it goes.
- **Say the cause only when the reader can act on it.** "lore scans only while
  its window is open" earns its place on an empty range, because opening the
  window is the fix. "The collector ships inside this app" does not.
- **A floored count says `at least` on the figure, never in a footnote below it.**
  Coverage for file changes is incomplete by construction — three saves inside
  one scan interval leave one mtime — so the number means something different
  from what it appears to, and a caveat under the list is a caveat the eye skips.
  Two words on the number are shorter and harder to miss.
- **Two panes describing one thing use one wording.** Detail and the period panel
  swap in the same slot; Stream and Detail describe the same commit tiers. Every
  such pair was phrased twice, and two phrasings of one claim read as two claims.
- **Em-dashes belong in this document, not in the interface.** Where a sentence
  needs two clauses in a pane, it takes a period or a semicolon.

Section headings across the panes are plain nouns — Sessions, Commits, File
changes, Projects, Evidence, Recorded — not questions. "Where it went" and "How
it is known" were good writing and the wrong register beside a list of counts.

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
heading than below it. The timeline's rows follow the grain: a project row at day
grain is 42px, a day row at week grain is 44px, a day tile is at least 82px, and
a day cell is 8px square on a 2px gutter. One `--edge` (16px) governs the hour
axis and every mark under it, so a label and the mark below it cannot drift apart
— they did once, when the axis had no indent and the rows had sixteen pixels of
it, which put every gridline sixteen pixels right of its own tick.

The fixed project gutter that used to sit beside the track is gone. It cost
`clamp(112px, 16vw, 172px)` of a centre pane that holds 664px with both side
panes open — a quarter of the axis spent on a name that fits on a line of its
own — and the day rungs now set names above their track instead. With the detail
pane folded on arrival, the timeline went from 492px to 1036px at a 1280px
window.

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
`border-radius` literal remains in the build. One did survive the sweep — the
2px on a tile's micro strip, which is `--radius-mark` spelled out — and a claim
of "none" with one left is worth less than the sweep was. Two structural constants are named
for the same reason: `--traffic-inset` and `--traffic-inset-wide` are where macOS
puts its window controls, measured for two bar heights, and they move together.

Reading measure is capped at 78ch in the conversation reader, centred, regardless
of window width.

### Range bar

Navigation and context in one line above the timeline, and **rendered independently
of the digest** — a range holding nothing still has to say which range it is and
still has to let you leave.

`‹ Mon 11 – Sun 17 Aug ›`, plus **Back to now** only when there is somewhere to
come back from, plus a scan indicator only when the rail that normally carries one
is folded. Forward is disabled at the present: there is no record of the future.

**The Unit and the Instance Rule.** The rail selects the *unit* — by day, by week,
by month, all time — and the range bar selects *which one*. That split is why the
rail's labels are no longer "Today" and "This week": those were true only while the
range sat on the present, and a range you can step makes them lies four fifths of
the time. Changing the unit returns to the present, because "three back" means
something different per unit and silently reinterpreting it is how a range stops
being trustworthy.

### Shortcut sheet

`?` from any surface. A sheet, not a modal: it protects no task and interrupts
none, so it dismisses on any click and needs no confirmation. It takes focus on
open and returns it on close, because a sheet that announces nothing is a scrim.

Its contents are a **hand-maintained list beside the key handler, not derived from
it.** They agree today; nothing enforces that they keep agreeing, and a binding
added to one and not the other is the failure mode to watch. Deriving the sheet
from the handler — or the handler from the sheet — is the fix, and it has not been
made.

Thirteen bindings were reachable and three were discoverable, in tooltips, on
icons. An accelerator nobody can find is not an accelerator.

## Elevation & Depth

**Structural, not ambient.** Two devices, used for different jobs:

- **Real window vibrancy** (`NSVisualEffectView` via Tauri's `underWindowBackground`)
  on the scope rail *only*. The rail paints almost nothing of its own —
  `rgba(255,255,255,0.022)` — and lets the material through. Dense content never
  sits on a live background.
- **Shadow with offset and blur** on genuinely raised surfaces: detail cards, the
  selected block, the selected timeline mark. There is exactly one step,
  `lift-1`, and
  that is the whole vocabulary. A second step was defined for a surface floating
  above a pane; nothing in this product floats — the reader and settings *replace*
  the timeline rather than hovering over it — so it was deleted rather than left
  standing as a claim the system does not keep. Add a second step when something
  genuinely needs one, not in advance.

A third device exists for exactly one job: `scrim`, the dim behind the shortcut
sheet. It **covers** rather than lifts, and it is a dim rather than a blur — the
window spends its one blur on the rail's vibrancy, and a sheet needs separation,
not atmosphere. Nothing else in the product covers the window, so nothing else
uses it.

1px dividers at 7% white carry structure everywhere else. A raised surface changes
its shadow, not just its border; a surface that only changes its border is not
raised.

Glass is a material with one job here. Blur applied for decoration, or vibrancy
extended to content panes, breaks the rule that separates the rail from everything
else.

## Shapes

Radii: `9px` for panes, block cards and control groups; `6px` for controls and
inner cards; `4px` for progress and share bars; `3px` for category swatches and
day cells; `2px` for timeline marks and
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
  overlaps counted once, across-projects is the sum and may exceed the range —
  which the two labels say by themselves, so neither figure carries a gloss under
  it any more. The
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
- **Timeline marks** carry evidence in their texture and project class in their
  hue. See below.
- **The right pane answers whatever the rung selects.** A day and a week draw
  blocks, so it describes a block. A month and all time draw days and months, so
  it describes one of those instead: the period's total, its projects ranked by
  share, the evidence split that adds up to that total, and what was recorded.
  Before this it asked the reader to select a block on screens that draw none —
  372px spent on an instruction that could not be followed. Everything it shows
  is counted from the lanes already on screen, never re-queried, so it cannot
  disagree with the marks beside it.
- **A click selects; it never navigates.** Every mark on the timeline answers in
  the pane, and clicking the same one again clears it. Moving the window is a
  separate, deliberate step — the pane's own "Open this day" or "Open this
  month". A tile that jumped the whole window on a single click made the
  cheapest gesture the most disruptive one.
- **The keyboard walks what the rung drew, in the order it drew it.** `j` and `k`
  step whatever the range resolves to — a block at day and week grain, a day at
  month and all time, a month on the contact sheet — and the timeline publishes
  that list rather than the page reassembling one from the lanes. Two
  consequences the page's own copy got wrong: a walk through the week rung now
  goes newest day first, the order the rung actually draws, instead of sorting
  every bar in the range by start time and contradicting the axis; and the last
  press of `j` holds the final mark instead of clearing it, which is what a
  clamped step onto a toggle used to do. A walked mark is scrolled into view
  through a selector that lives beside the type it matches, because the attribute
  and the lookup drifting apart is silent, and did.
- **The rail's project list is the ranked view, and it is uncapped on purpose.**
  No rung of the ladder lists projects as rows, so this is the only place they
  are enumerated: the durations it already carried gain a share bar behind the
  label, and the list answers "where did it go" while the ladder answers "when".
  A cap was written here once, at the twelve busiest, and taken back out —
  because a list that is the *only* enumeration of a thing cannot be truncated
  without removing a capability. A project past the cut would have been
  unselectable from the one surface that offers projects at all, which is not a
  tidier ranking, it is a missing filter. The list is also the wrong shape for a
  cap: Stream bounds at 300 because a block card is heavy to render a thousand
  times, whereas a rail row is one flex line, already ordered by the quantity you
  came to compare, in its own scroller. Length costs a scroll here, not an answer.
- **Empty and partial states** name what they hold rather than rendering blank: a
  block with nothing itemised states its record count; a continued session states
  where it began and that its figures count once.
- **Motion**: one authored moment per surface. Timeline marks are revealed from
  their own start along the axis, staggered down the rows; day tiles settle in
  place rather than sliding, because a grid that arrives in sequence reads as a
  loading state; the reader arrives with a 260ms settle from an already-visible
  default. All on `cubic-bezier(0.16, 1, 0.3, 1)`, all disabled under
  `prefers-reduced-motion`. The mark reveal animates `clip-path`, not `scaleX`:
  scaling squashed the end marks of a saves-only span into slivers and stretched
  the hatch pitch, distorting precisely the two treatments that carry the most
  meaning. An entrance that deforms its own content is not an entrance. Every animation *and* every state
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

The system's other signature. A block's start and end are the timestamps of its
first and last record, so *what its width means* changes with what those records
are. Drawn identically, a three-hour conversation and two file saves make the
same claim and only one of them has earned it.

Four treatments, one descending scale of how much of the span is actually known.
They are drawn as **texture over the mark's own hue**, so a day row can say whose
work a span was and how well it is known at the same time. Selection recolours
without flattening what a mark is:

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

**The Ink Rule.** Selection takes no ink at all: it recedes everything it was not.
The accent was the obvious choice and turned out to be unavailable —
`accent-magenta` and `category-research` are the same value, so a selected rose
mark was indistinguishable from an unselected one, and a ring in the same magenta
failed the same way. Both other channels were already spoken for: hue says which
project, texture says how the span is known. So the chosen mark is left exactly
as it was and every other mark on the timeline drops to
`brightness(0.42) saturate(0.75)`, which works identically on every category
because it depends on no hue.

**The recede excludes the chosen mark; it is never undone by a rule after it.**
`:not()` carries the specificity of its own argument, so
`.picked .tile:not(.pad):not(.empty)` weighed four classes against
`.picked .tile.on`'s three and won outright — selecting a tile dimmed every tile
including the one selected, which is the exact inverse of what this rule is for,
and it read as the grid greying out for no reason. Every recede now excludes
`.on` in the rule that dims, so there is nothing to out-weigh and nothing that
breaks if the rules move.

Dim with a filter, never with opacity — **on a span**. At 28% alpha the hour rules
showed through every bar and two marks that touched blended into a third colour;
a filtered mark stays opaque and still hides what is behind it. That reasoning is
about what sits *behind* the mark, so it does not reach the two rungs where
nothing does. A tile and a cell sit on bare ground, and a cell recedes on the one
property it is already spending — its own density alpha, scaled — because a
filter starts a compositing layer per element and a year sheet holds 2,697 cells,
thirty times the eighty-nine panels whose filters already had to come out for
stutter. Tiles keep the filter: forty-five of them cost nothing, and their
entrance animates opacity and would win the cascade against it. **The mechanism
follows what is behind the mark and what else is animating it; the meaning is the
same recede either way.** The selected mark also rises above
its neighbours — a day strip carries every project at once and the archive holds
403 cross-project overlaps, so without it the chosen mark came out striped by the
marks it was chosen over. Any state that flattens a treatment, or spends a channel
already carrying meaning, has destroyed the thing it was there to say.

**The Legible Mark Rule.** Every mark that carries a class clears 3:1 against
`ground` on its own — solid 3.28, hatch strokes 4.22, amber caps 9.79, connector
3.34, selected connector 3.15. A hatch *bed* is exempt because its strokes carry
identity, but a connector is not: without it, two end marks are two unrelated
dots. The category fills this replaced measured 1.60–2.25 and all failed.

Below 14px of rendered width the treatments stop being treatments — a hatch reads
as noise, a pair of end marks touch — so a narrow bar keeps its class colour and
gives up its texture. The lane legend names only the classes the range actually
holds, and always renders at least one. Suppressing it below two classes had the
logic backwards: a day evidenced only by file saves is exactly when someone meets
an unfamiliar treatment cold, and it was exactly when no key was drawn.

**And only where a mark can carry a treatment at all.** Evidence rides the
surface of a span, and only the two rungs that draw spans have a surface — a
tile's micro strip and a cell carry hue and nothing else. The key was rendered
outside the rung chain, so at month and all time it named four treatments that
nothing on screen could show, teaching a vocabulary the screen was not speaking.
Each half of the key is now drawn only where it names something: the treatments
where marks have texture, the hues where a row is not already headed by its
category, the day count wherever a range is longer than a day. The class also rides the bar's `title`, which is reused
verbatim as its `aria-label`, so the one channel the shape cannot reach gets it.

## Do's and Don'ts

**Do**

- Show how a claim was established, next to the claim.
- Keep magenta for selection and live state; keep amber for uncertainty.
- Set data in the mono token with tabular numerals.
- Say what a surface does hold when it cannot show what was expected.
- Write the shortest thing that is still true, and put a qualifier on the figure
  it qualifies rather than in a note beneath it.
- Cap a view by relevance, not by chance, and state the remainder: Stream keeps
  the most recent 300 blocks and says so. Cap it only where the cost is real,
  though — see the Don't below.
- Let the surface that draws the marks be the one that says which marks exist.
  Anything else — the keyboard, the pane, the legend — reads that list rather
  than deriving its own.
- Give a raised surface a shadow with offset and blur.
- Let a mark that carries meaning clear 3:1 against its ground on its own.
- Split a figure only where the parts add up. Elapsed does not; across-projects
  does.

**Don't**

- Set interface type below 13px or below weight 500. Reading prose is the one
  exception and sets at 400.
- Use the accent as a glow, a gradient, or a decorative fill.
- Extend vibrancy beyond the scope rail, or add blur for atmosphere.
- Print a figure lore cannot support — no `0m` for a real block, no clock range
  without dates across days, no total where the data only supports a floor, no
  digest over a range holding nothing.
- Introduce a second accent, or let a category hue carry state.
- Let hue carry state anywhere on the timeline. Project class may tint a mark,
  because a day row holds several projects and the mark has to say whose work it
  was; selection and live state stay magenta, and no third meaning joins them.
- Draw a class-carrying mark below 3:1 on ground, including at the bottom of a
  density ramp. The floor is the design, not the exception.
- Draw a date cell that carries nothing but its date. A cell earns its place by
  holding real hours and a real class; a grid of positions with no evidence in it
  is the metaphor the anti-reference was written against, and revoking the ban
  did not revoke that.
- Borrow a word or a colour across the attribution and evidence axes.
- Explain lore's architecture in a pane. The reader came for an answer, not for
  how the answer is built; that belongs here.
- Follow a heading with a sentence that restates it.
- Leave a figure unlabelled about what it covers. The digest is narrowed by the
  rail's filters in SQL; the one narrowing it cannot follow is a text query, and
  it says so.
- Ship an uncapped view *whose length costs the reader an answer*. The timeline
  is bounded by the range itself — a grain always resolves to a finite set of
  days — and Stream states its 300. But a cap is a subtraction, and it has to be
  paid for: **never cap the only enumeration of something.** The rail's project
  list is the sole place projects appear as rows, so bounding it would delete a
  filter rather than shorten a list, and it stays uncapped however long a range
  makes it. A scroll is the right price for length; an unreachable project is
  not.
- Draw a mark the keyboard cannot reach, or let the keyboard reach one that was
  not drawn. Both are the same fault seen from opposite ends, and both come from
  a second copy of the rule that decides which rung is on screen.
- Name a channel in a legend that the marks on screen do not carry.
- Spend the accent on a row type, a role, or a category of thing. It means
  *chosen*, and one more meaning is all it takes to mean nothing.
- Animate a bar with a transform that deforms the marks inside it.
