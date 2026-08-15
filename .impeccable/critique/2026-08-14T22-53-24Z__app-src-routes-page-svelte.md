---
target: the lore main window
total_score: 22
max_score: 40
na_heuristics: 
p0_count: 2
p1_count: 3
timestamp: 2026-08-14T22-53-24Z
slug: app-src-routes-page-svelte
---
Method: dual-agent (A: design review, isolated · B: detector + mechanical evidence, isolated). Assessment A completed before any detector output entered synthesis.

Browser overlays: not available. No browser-automation tool exposed, no Playwright/Puppeteer/Chromium installed, and the surface needs Tauri IPC to render data. No live server started, no injection attempted. All findings are source-verified.

Target: `app/src/routes/+page.svelte` and its twelve-component tree. Mode: Operate.

## Design Health Score

| # | Heuristic | Score | Key issue |
|---|---|---|---|
| 1 | Visibility of System Status | 2 | Loading gated on `!summary` (`+page.svelte:520`); every refetch after the first is silent; detail pane shows the previous block while the next loads; zero `aria-live` regions. |
| 2 | Match System / Real World | 3 | Vocabulary is product-true and excellent — but "Run `lore scan` in a terminal" (`:518`, `:531`) names a command not on PATH. |
| 3 | User Control and Freedom | 2 | `removeRoot` auto-saves with no confirmation (`Settings.svelte:65-70`); Category has an "Everything" escape, Projects has none (`Rail.svelte:109-124`). |
| 4 | Consistency and Standards | 3 | Token discipline on color/radius/type is high; `duration()` "2h 05m" and `compactDuration()` "3.4h" sit adjacent in the digest. |
| 5 | Error Prevention | 1 | The window key handler's typing-guard misses every Settings input — the three numeric settings cannot be typed. |
| 6 | Recognition Rather Than Recall | 2 | 14 keyboard bindings; 3 exist only in hover `title`s; 11 documented nowhere. |
| 7 | Flexibility and Efficiency | 3 | Strong for the archetype, undermined by an unbounded query on every selection and no `scrollIntoView` anywhere. |
| 8 | Aesthetic and Minimalist Design | 3 | Real restraint, disciplined palette. Lapses: the flat six-metric digest, raw collector stdout in a `<pre>` (`Settings.svelte:271`). |
| 9 | Error Recovery | 2 | The crash panel (`:504-513`) is exemplary. The archive-read error four lines below offers no retry — only the impossible terminal command. |
| 10 | Help and Documentation | 1 | No shortcut sheet, no onboarding, no definition of "block" anywhere in the main window. |
| **Total** | | **22/40** | **Acceptable** — significant improvements needed |

All ten apply; none marked n/a.

## Design Specificity Verdict

**Authored at the leaf, interchangeable at the trunk.**

LLM assessment. lore's identity is real and almost entirely in the annotation layer. The confidence grammar is a genuine design-system component: color (`--add`/`--text-dim`/`--amber`), a purpose-drawn icon pair — closed ring for witnessed, the same ring broken for inferred (`Icon.svelte:63-70`) — a short form in the dense view (`Stream.svelte:202-206`), and a full sentence with its reasoning in the inspector (`Detail.svelte:77`). Four registers, one meaning, scaled to available space. Same for the honesty flags that name the adversary (`prompts only`, `only in lore`) and the save-moment spine in `Moments.svelte`.

But the two surfaces you land on — the digest and the lanes — have no lore in them.

The digest sets `elapsed` (wall-clock derivation), `commits` (exact git count) and `% AI-written` (inferred attribution built on the same certain/strong/weak tiers the app grades obsessively) in identical mono, weight and color, side by side (`+page.svelte:485-500`). Three epistemic classes as peers. The evidence grammar evaporates exactly where the Insight job lives.

The lanes view is a Gantt chart whose bars encode category — the least surprising fact about a project, already stated three times (gutter swatch, group heading, rail row). A bar from one file mtime and a bar from a witnessed three-hour transcript are the same object, differing only by width. The one attempt to fix this, `class:has-commits` at `Lanes.svelte:161`, has NO CSS rule anywhere (verified: zero matches in source and compiled stylesheet).

Repointed at Jira tickets or Toggl entries, the shell, toolbar, rail, digest and lanes would all survive unchanged.

Deterministic scan. The detector first ran DEGRADED — `htmlparser2`, `css-select`, `css-tree`, `domutils` missing from the skill install, so custom properties, selector matching and computed contrast were not evaluated. Assessment B reinstalled deps into a scratch tree and re-ran non-degraded with suppression disabled (`--no-config --no-design-system --no-inline-ignores`) plus a deliberately-bad sanity file to prove the scan wasn't a no-op.

Result: 0 findings across all of `app/src` — genuinely clean, not suppressed. The 46-rule set includes `tiny-text`, `undersized-ui-text`, `low-contrast`, `pulsing-dot`, `clipped-overflow-container`; none fired.

One finding on compiled CSS only: `[side-tab]` on `border-left: 2px solid var(--line-strong)` from `Markdown.svelte:131`. FALSE POSITIVE, confirmed — the rule targets a thick colored border on a card; this is a neutral divider token on a `<blockquote>`.

Where the clean bill is narrower than it looks: the static rules evaluate declared values, not what they compute to across `em` inheritance, alpha compositing, or cross-component overflow ancestry. By hand: 24 rendered instances below the 13px floor, two rules that delete the focus outline and win on specificity, one real text-contrast failure at 4.47:1, and focus rings clipped by an ancestor's `overflow: hidden` — one instance each of four rules that reported nothing.

Visual overlays: none. No browser automation available; no user-visible overlay exists. Fallback signal is the CLI scan above plus hand-computed WCAG math.

## Overall Impression

A serious build. The reasoning in the source comments is better than in most design docs; the empty-state vocabulary refuses to render blank in three distinct ways; the token system is genuinely disciplined.

It is also one careless line away from being much better in four places, and has two bugs on the happy path that produce confidently wrong answers — the exact failure `+page.svelte:130-134` says the app exists to prevent.

Biggest opportunity: move the evidence grammar from the leaf to the trunk. The digest and the lane bars are where lore's argument should be loudest and where it is completely absent.

## What's Working

1. The confidence grammar as a scaled system — four registers, one meaning. The pattern the digest and lanes should be extended with.
2. Progressive disclosure that states its own size. Every cap names what it hid: `Markdown.svelte:36-38`, `Moments.svelte:52-56`, `+page.svelte:551-556`, `Stream.svelte:248-252`. No truncation masquerades as completeness.
3. The keyboard-and-markup baseline. Not one `div`/`span` with a click handler in `app/src` — every `onclick` is a real `<button>`, which is why `tabindex` appears zero times. Every SVG carries `aria-hidden` and `focusable="false"`. All ten icon-only buttons correctly labeled. All three animations `prefers-reduced-motion`-guarded; exactly one `infinite` animation exists and it's the collector dot.

## Priority Issues

### [P0] Single-key shortcuts fire inside Settings' inputs — the numeric settings cannot be typed

`+page.svelte:395` binds `onKey` to `svelte:window`. The typing guard at `:334` is `document.activeElement === filterField` — and `filterField` is null whenever Settings is open, because the toolbar binding it lives in the `{:else}` branch of `{#if settingsOpen}` (`:422-426`). The bail-out at `:359` never triggers for a Settings field. Found independently by both assessments.

- Typing `30` into "Scan every ___ minutes" (`Settings.svelte:141-147`): `3` hits `scopeKeys` at `:361-368`, calls `preventDefault()`, silently switches range to This month, never reaches the field. You see `0`.
- `1`, `2`, `4` swallowed identically in all three timing fields.
- Arrow up/down — the native increment — are `preventDefault()`ed and walk the timeline selection behind the settings screen.
- `j`/`k` cannot be typed into category or identity fields; `s`/`l` type but also switch the hidden view; `Enter` fires `openSession`.

Why it matters: the three most consequential settings — scan interval, idle gap, first-scan lookback — are untypeable, and the range changes underneath you.

Fix: test element type, not identity, and bail before any `preventDefault`.

Suggested command: /impeccable harden

### [P0] Clicking a lane bar shows the previous block's detail while an unbounded query runs

`+page.svelte:226-241`. In Lanes view `blocks` is always `[]` (`:210-212` fetches the timeline only for `wanted === 'stream'`), so `blocks.find(...)` at `:229` never hits. Every bar click falls to `:235`: `archive.timeline(fromMs, toMs)` with NO limit. Over "All time" that's the entire block table. Verified: `archive.ts:217` accepts `limit`, `project`, `category`; the call site passes none.

`selectedBlock` flips at `:227` so the bar turns magenta immediately, while `selectedDetail` is replaced only on resolve. `Detail.svelte` has no loading branch — `{#if !block}` at `:16` is the only guard, and `block` is still the old one.

Why it matters: the primary interaction of the primary view renders a confident inspector describing a different project than the bar just highlighted. Three fast clicks produce three overlapping fetches; last to resolve wins, not last clicked. No cancellation.

Fix: null `selectedDetail` before the await, render a loading branch in `Detail.svelte`, add a `block(id)` command so one selection doesn't refetch a range.

Suggested command: /impeccable harden

### [P1] The product's one hard guarantee has no alarm — and failed scans are invisible

PRODUCT.md:56 states the requirement lore depends on: run at least once inside each source's ~30-day retention window or that history is lost for good. The interface renders that as `Rail.svelte:308-317` — a 7px dot that is `--add` green by default, amber only when `lastScanMs === null`. An archive last scanned 25 days ago, five days from permanent loss, shows a green dot beside "Scanned 25d ago · every 60m."

Compounding: `collector.error` (`collector.svelte.ts:22`) renders only inside Settings (`Settings.svelte:267-269`). A scheduled scan failing while you're on the timeline displays nothing. `statusError` has the same shape — `+page.svelte:520-524` renders it only during initial load, so a persistently failing status poll (frozen health readout) is invisible.

Why it matters: the interface is most confident at the moment of greatest risk, and its health indicator cannot report its own failure.

Fix: make the footer a retention budget, not a timestamp — "5 days of source history left before Claude Code deletes it" — with amber and magenta earning their reserved meanings. Surface `collector.error` in the timeline footer. Make the footer a button that triggers the scan it reports on.

Suggested command: /impeccable clarify, then /impeccable harden

### [P1] The three states that explain absence are the least trustworthy surfaces in the app

1. The recovery instruction is impossible. `+page.svelte:518` and `:531` both say "Run `lore scan` in a terminal." Per PRODUCT.md:20 the collector "ships inside the app bundle and is never installed anywhere." A working scan trigger sits 20 lines away in `Settings.svelte:245-251`.
2. The empty state contradicts what's happening. On first run `:530-531` says "The archive is empty. Run `lore scan`" while `collector.watch` has already started one (`collector.svelte.ts:59-66`: `lastScanMs === null` → `elapsed = Infinity` → `due = 0`). Only counter-evidence is a pulsing 7px dot, absent if the rail is collapsed. A first scan over 52 GB / 307K files has no progress, counter, ETA, or cancel.
3. The empty state is blind to the filters that caused it, and makes a false claim. `:525-538` branches on `query` and `records === 0` only; `category` and `project` are never mentioned, and a scope change clears `selectedBlock` but not the filters (`:412-416`). Filter to a project, press `1`, and the app blames the archive for a filter. Worse: `filteredBlocks` narrows the already-capped 300 blocks, and in Lanes view `blocks` is `[]`, so typing any commit subject returns "No project, session or commit matches" — a statement the data does not support, in a product whose fifth Don't is "print a figure lore cannot support."

Fix: replace both terminal instructions with a Scan now button; show a live record counter while busy. Enumerate every active narrowing in the empty state with a one-click clear. Change no-match copy to state its reach: "No match in the 300 most recent blocks in this range."

Suggested command: /impeccable onboard, then /impeccable clarify

### [P1] Two normative rules, broken by four CSS lines

Both declared binding in DESIGN.md and the brief; both fail in the compute rather than the declaration, which is why the detector reported clean.

Keyboard focus is invisible on every text input. `+layout.svelte:153-157` defines a correct global ring (2px accent, 2px offset; 4.16–5.20:1 on every surface). Two rules delete it and win on specificity — Svelte scoping compiles them to (0,2,1) against the global's (0,1,0):
- `Settings.svelte:459-462` — `input:focus { outline: none; border-color: var(--accent-edge) }`. Replacement composites to `#6a234b` = 1.77:1. Fails 3:1. Also `:focus`, not `:focus-visible`, so it fires on mouse click.
- `+page.svelte:686` — `.filter input { outline: none }`, unconditionally. The `:focus-within` border replacement measures 1.77:1.

All six text inputs have no perceptible keyboard focus indicator. Separately, `Stream.svelte:334-340` sets `article { border-radius: var(--radius); overflow: hidden }` while children `button.head` and `button.item` declare neither — the global ring at `outline-offset: 2px` is clipped on the left, right and top edges of the primary selection targets in Stream view.

The 13px type floor is violated in 24 rendered places by one unguarded line. `+layout.svelte:184` — `:global(.num) { font-size: 0.92em }` — and `+page.svelte:781` — `code { font-size: 0.92em }`. Inside containers at 13px/13.5px these compute to 11.96px and 12.42px:
- `Detail.svelte:42-46, 66-69` — every session and commit figure, inside `.figures` @ 13px → 11.96px at weight 600
- `Stream.svelte:157-159, 189-191, 298` → 11.96px
- `Reader.svelte:41, 44, 50-52, 75-77` → 12.42px and 11.96px
- `+page.svelte:553-554` (truncation notice) → 11.96px
- `+page.svelte:518, 531` (`<code>lore scan</code>`) → 12.88px

These land on the exact data figures DESIGN.md singles out. The fix already exists in the codebase: `Spans.svelte:21` uses `max(13px, 0.92em)` with a comment naming this hazard. Solved locally, never applied to the global helper.

Fix: delete both `outline: none` declarations; give the Stream article `overflow: visible` or move the radius to children; change `.num` and `+page.svelte:781` to `max(var(--fs-min), 0.92em)`.

Suggested command: /impeccable audit, then /impeccable polish

## Persona Red Flags

Alex (impatient power user — the actual archetype). Every lane-bar click stalls on an unbounded query with no spinner and a stale inspector; three fast clicks race. The entrance animation replays on every filter keystroke — `+page.svelte:540` keys `<Lanes>` on `${scope}-${category}-${query}`, so typing "lore" remounts four times, each replaying the 620ms `grow` with up to 264ms stagger; `project` is not in the key, so project filtering doesn't replay. 14 shortcuts, 3 discoverable via hover only. `Enter` opens `selectedDetail.sessions[0]` — for the 43 non-git non-Claude directories that's the common case of a block with no sessions, and Enter does nothing, silently. `j`/`k` change `selectedBlock` but never move focus and never scroll (`scrollIntoView` appears zero times, verified), so walking a long list selects off-screen. The rail footer — the only health readout in a trayless app — is not a button.

Sam (keyboard-only + screen reader). The primary search field has NO accessible name: `+page.svelte:447-461` wraps it in a `<label>` whose only other children are an `aria-hidden` icon and a `<button>`; buttons are excluded from name computation, so it announces as "edit text, blank." Same at `Settings.svelte:216-220`. The view toggle uses `class:on` with no `aria-pressed` (zero occurrences app-wide). The rail's scope rows set `aria-current`; its category and project rows don't. Zero `aria-live` regions: nothing announces scan started/finished, the archive reloading under you (`:114-117`), loading, or a filter narrowing 300 blocks to 2. Opening Reader/Settings unmounts the focused element with no focus management. Load-bearing information is hover-only — both digest time definitions, all three tier explanations, the continued-session rule, both honesty flags exist solely in `title`. No `<h1>` in the main window. The primary control of the primary view — the lane bar — is 23px tall and floors at 3px wide (`Lanes.svelte:37-38`, `:318`). The clear-filter × is 16px tall (`+page.svelte:694-698`: `display: grid` with no height, doesn't stretch to its 30px flex parent).

Riley (stress tester). Stream is capped at 300 with an honest notice; Lanes is not capped at all — `archive.lanes()` takes no limit (`archive.ts:225`) and `Lanes.svelte:157` renders every bar of every lane with its own animation. The landing view is the unbounded one. `Detail.svelte:100` passes `<Moments changes={block.file_changes} />` with no `limit` — and `Moments.svelte:8-15` implements a `limit` prop with a "+N more" summary nothing ever passes (verified: single call site). 5,000 mtimes render 5,000 rows into a 372px column. A 90-character project name breaks twice: `Stream.svelte:364-368` sets `.project` at 15.5px with no `min-width: 0`, no overflow, no ellipsis, pushing the duration past the edge of an `article` with `overflow: hidden` — the duration vanishes; `Detail.svelte:154-159` has no `overflow-wrap` on the 21px `h2`, unlike `.title` at `:233`. The correct pattern exists in `Rail.svelte:256-261` and `Lanes.svelte:288-295`. On "All time" over the documented 7-month range, `Lanes.svelte:66-74` steps the tick cursor by 12 months from January 1 of the earliest year: the axis renders exactly one tick, "2026", at position 0. Between 880px and 1120px — a band including `tauri.conf.json`'s own `minWidth: 880` — `aside.detail` is hidden by CSS while `detailOpen` stays true, so `select()` sets it to no effect, Shift-Cmd-B does nothing visible, and block detail is completely unreachable at the app's own minimum window size.

## Minor Observations

- `--lift-2` defined (`+layout.svelte:79`), used zero times, though DESIGN.md names it as one of two elevation devices.
- `Settings.svelte:259-265` — the `{:else if saved}` branch suppresses "Saved" whenever `needsScan`/`needsRebuild` is true. Also renders in the Collector group, potentially far below the field edited.
- Four transitions have no `prefers-reduced-motion` guard (`+layout.svelte:138`, `+page.svelte:649`, `Rail.svelte:239`, `Stream.svelte:586`). All three animations are guarded; no global reset.
- Spacing has no tokens at all: DESIGN.md declares 2/4/7/11/18/26; none has a CSS variable; ~79% of 222 spacing declarations are off scale. The four most-used values — 10px (28x), 8px (24x), 6px (21x), 12px (18x) — are all off-scale and outnumber the entire on-scale set.
- DESIGN.md says the shell is a CSS grid at `244px | minmax(0,1fr) | 372px`. It's flexbox; `minmax(` appears nowhere. Behaviorally equivalent, mechanism wrong. Similarly DESIGN.md prose says "nothing lighter than 500" while its frontmatter sets `body.fontWeight: 400`, and the build follows the frontmatter (`Reader.svelte:408-412`, `Markdown.svelte:86-91`).
- DESIGN.md claims `text-faint` is ~5.2:1 on ground. Measured 5.95:1. 5.20 is `--accent`'s ratio.
- `Reader.svelte:369-371` — `.badge.wrote`, accent on accent-soft at 13px/580 — measures 4.47:1, the one genuine text-contrast failure in real use.
- `+page.svelte:83-85` — `toMs` is a ternary with two byte-identical branches. `select()` accepts `projectPath` and discards it (`:240`).
- `Stream.svelte:137` calls `entriesOf(block)` (sorts + clusters) inside the template — recomputes for all 300 blocks on every re-render.
- `Stream.svelte:64` keys file-moment entries on array index while `openMoments` keys on the same index — disclosure state mis-associates when the set changes.
- `Rail.svelte:77` hides the Category section when `categories.length <= 1`, so a single-root user never learns categories exist.
- `Moments.svelte:22` puts `aria-hidden="true"` on the whole gap row, removing "23m later" from the accessibility tree.
- The detector's own dependencies are missing from the skill install, so it silently ran degraded on first invocation.

## Questions to Consider

1. Why is the digest — the first thing read, and the only surface serving the Insight job — the one place with no confidence grammar? What would "3h 20m elapsed" look like broken into the fraction backed by transcripts, the fraction backed by commits, and the fraction that is a floor built from mtimes?
2. What if the lane bar's fill encoded evidence density instead of category — solid for witnessed, hatched for inferred, ghosted for mtime-only — and category retreated to the gutter where it's already stated twice? `has-commits` is a dead stub of exactly this.
3. What if the rail footer were a retention budget rather than a timestamp?
4. Both landing views answer "when." Neither answers "what." File provenance has no surface in the window at all. Is Recall actually a time question?
5. The best writing in the product is locked behind Cmd-comma. Why does the person staring at an empty archive get "Run `lore scan` in a terminal" while the person who found Settings gets an honest paragraph?
