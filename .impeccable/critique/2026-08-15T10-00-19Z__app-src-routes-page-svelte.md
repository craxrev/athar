---
target: the lore main window
total_score: 27
max_score: 40
na_heuristics: 
p0_count: 1
p1_count: 3
timestamp: 2026-08-15T10-00-19Z
slug: app-src-routes-page-svelte
---
Method: dual-agent (A: design review, isolated · B: detector + mechanical evidence, isolated). Assessment A completed before any detector output entered synthesis.

Browser overlays: not available. No browser-automation tool, no Playwright/Puppeteer/Chromium, and the surface needs Tauri IPC. No live server started, no injection attempted. All findings source-verified.

Target: `app/src/routes/+page.svelte` and its component tree. Mode: Operate.

IMPORTANT: Assessment A measured the build BEFORE the final fix batch in this run. The score below is what A saw. Items marked FIXED-AFTER were addressed after measurement and are not reflected in the number.

## Design Health Score

| # | Heuristic | Score | Key issue |
|---|---|---|---|
| 1 | Visibility of System Status | 3 | Strong instrumentation, but during a first scan the stage said "lore may not have been running" the instant records exceeded 0. FIXED-AFTER |
| 2 | Match System / Real World | 3 | Vocabulary outstanding; `elapsed` vs `across projects` explained only in hover title |
| 3 | User Control and Freedom | 3 | Escape ladder, filter chips, restore-focus all good; no confirm on removeRoot; no rail+detail together below 1120 |
| 4 | Consistency and Standards | 3 | Tokens disciplined, radius literals gone; the live region contradicted the visible empty state. FIXED-AFTER |
| 5 | Error Prevention | 3 | Revision-checked saves, ticketed selection, disabled buttons, idempotent scans. Genuinely strong |
| 6 | Recognition Rather Than Recall | 2 | Eleven key bindings, three hinted, none in a reference surface; legend suppressed at one class |
| 7 | Flexibility and Efficiency | 2 | No way to reach a specific past day — fromMs derives purely from now; j/k never scrolls into view |
| 8 | Aesthetic and Minimalist Design | 3 | Restrained and committed; the digest can still present ten parallel figures |
| 9 | Error Recovery | 3 | Four independent fault channels that refuse to mask each other; a stale digest survived above the error panel. FIXED-AFTER |
| 10 | Help and Documentation | 2 | No shortcut reference; the evidence code had no persistent explanation. PARTLY FIXED-AFTER |
| **Total** | | **27/40** | **Good — address weak areas** |

All ten apply; none marked n/a. Previous run: 22/40.

## Design Specificity Verdict

**Grounded, but the grounding is concentrated in the leaves and thins toward the shell.**

Authored for lore and not liftable: the four evidence treatments on the lane bars; the rail footer's retention framing ("5 days of source history left"), which is the single most product-native sentence in the build; the honesty flags (`prompts only`, `only in lore`, the file-count floor caveat, `continues from`); Settings' explanation of why there is no daemon.

Category-interchangeable: the three-pane shell and its five-unit toolbar; the rail; Stream's day-headed activity feed; the digest AS A FORM (its content is specific, its shape is a KPI strip).

On the evidence system specifically — genuine, and structurally so. `evidence_of` (stats.rs:17-29) is consumed by BOTH the lane query (api.rs:409) and the range summary (stats.rs:319), so a bar and the digest cannot classify the same block differently. The split divides project_ms, whose parts sum exactly, rather than elapsed_ms, whose parts would not. That is a visual language enforced at the data layer instead of by convention.

Deterministic scan: the bundled detector runs DEGRADED (missing deps). A non-degraded copy was built in scratch space and confirmed at 0 bytes stderr across six runs. Result: ZERO findings on app/src, on +page.svelte, on app/src/lib, and under both --scope layout and --scope type. One finding on compiled CSS: [side-tab] on a blockquote's 2px --line-strong border — re-adjudicated FALSE POSITIVE (not a card, not an accent, the conventional typographic form).

## Overall Impression

The strongest work in this build is not visual, it is structural: a classifier enforced in Rust so two surfaces cannot disagree, four fault channels that refuse to mask each other, and a truncation rule applied uniformly enough that "this app tells me when it is holding back" becomes a learnable property.

The biggest remaining opportunity is navigation. The product's own primary job is "what did I do on this day", and there is no way to reach a day that is not today.

## What's Working

1. The evidence class is enforced in Rust, not agreed in CSS. One function, two consumers, re-exported specifically so a second copy cannot exist.
2. The failure taxonomy: window crash, archive read, status poll, and single-selection read are four independent channels, plus a retry that refetches rather than dismisses, plus a request ticket so three fast clicks settle on the block asked for last rather than the one that answered last.
3. Caps that state their own size, in one voice: 300 of N blocks with "the archive holds all of it", "+N more changes in M later moments", "{lines} lines" on a collapsed block, "N more files were in this commit than the archive kept".

## Priority Issues

### [P0] The digest describes the whole range while the view is filtered — ADDRESSED
archive.summary takes no project/category/query argument and the Rust summary has none to give, unlike timeline and lanes. Filter the rail and every lane below changes while the 19px lead figure keeps describing every project in the range.
Addressed by labelling rather than filtering: a text query cannot be pushed to SQL at all, and half-filtering (some figures narrowed, some not) would be worse than none. The digest now states what it covers whenever a narrowing is active. Pushing project and category into the Rust summary remains the better fix and is recommended follow-up.

### [P1] No way to reach a specific past day or week — OPEN
fromMs derives entirely from Date.now() and toMs is pinned to the end of today. There is no offset, no previous/next, no date input. The only route to last Tuesday is All-time Lanes plus a click on a sliver. PRODUCT.md names "reconstructing what happened on a given day" as primary situation 1. Ruling out the calendar grid does not remove the need to navigate.
Fix: left/right to step the current scope one unit, print the resolved range beside the digest lead, disable forward at the present. One offset variable folded into fromMs/toMs, reusing the existing startOf helpers. NOT DONE — this is a feature, not a defect, and was left for an explicit decision.

### [P1] A stale digest survived above the "archive could not be read" panel — FIXED
The catch set error without clearing summary/lanes/blocks, so a screen saying it cannot read the archive kept quoting it. The catch now clears all three.

### [P1] Lanes fully remounted and replayed its entrance on every keystroke — FIXED
query was in the {#key}, so every character destroyed the subtree and replayed the 620ms staggered entrance, while project — which genuinely regroups the lanes — was missing. The key is now scope-category-project.

### [P2] The detail pane never named the evidence class the bar asserted — FIXED
BlockDetail now carries the same evidence stamp, and the pane states it in a sentence per class. The legend only renders above one class, so the moment a user met an unfamiliar treatment cold was the moment with no key on screen.

## Persona Red Flags

Alex (the actual archetype): cannot get to last Wednesday — four presets anchored to now. Eleven bindings, three hinted, only in title attributes. j/k walks the selection but never scrolls it into view. At tight widths, pressing j closes the rail out from under him.

Sam: j/k moves neither DOM focus nor the live region's content, so pressing it five times announces nothing. Lane bars carry a good aria-label but no aria-pressed, so the selected bar is announced identically to every other. In Lanes every bar is a tab stop in DOM order with no skip. The digest's two load-bearing definitions live only in title on non-focusable spans. Positive: focus is remembered and restored across the reader/settings takeover, both surfaces take focus on mount, Escape has a sane ladder, and the Stream ring is drawn inset to survive overflow:hidden.

Riley: empty archive correct and calm. Lanes has no cap at all — archive.lanes takes no limit parameter, unlike timeline, so the one view the user is told is safe for wide ranges is the uncapped one. A day of only file-mtime evidence draws amber end-caps with no legend. A 90-character project name clipped silently in Detail's h2 and shoved Stream's duration past the card edge — both FIXED-AFTER. At exactly 880px the panes are mutually exclusive and detail is reachable — verified.

## Minor Observations

- The stated weight floor was not held: body declared no font-weight, so 45 rules across 8 files rendered interface text at 400. FIXED-AFTER by setting the floor at the root and opting reading prose back down in the reader.
- ai_share double-counts overlapping files: ai_files counts distinct session_files paths and human_files counts distinct file_change paths, then divides by their sum, so a file both AI-written and mtime-recorded adds 2 to the denominator. OPEN.
- The filter placeholder promises "sessions, commits" in a view that searches neither.
- removeRoot deletes and saves with no confirmation.
- The retention warning erases itself the moment it becomes true: opening the window triggers a scan, which resets daysSince, so "5 days left" shows for seconds and then nothing records that a 25-day gap exists.
- Type floor: CLEAN. Both relative-unit sites are max()-guarded and resolve to 13px or above in every context.
- Motion: all 4 transitions and all 3 animations are reduced-motion guarded, per component. Verified in the compiled CSS.
- Accessible names: all 30 controls compute a name. Zero unnamed.
- Radius literals in app/src: ZERO. All 40 declarations use a token.
- Rust: cargo check 0 warnings 0 errors across three crates; cargo test 72 passed 0 failed.
- svelte-check: 41 errors, ALL inside node_modules (missing @types/node). Zero in app/src.

## Questions to Consider

1. The digest is a stat strip. Why does the flagship product of an "evidence over estimation" thesis lead with the one component shape that cannot carry evidence? The split was bolted underneath to fix that, and it works by proving the strip was the wrong container.
2. The retention warning erases itself the moment it becomes true. Should lore keep a scan history and draw its own coverage as a band along the lanes axis, so a stretch nobody archived is visually distinct from a stretch where nobody worked? The empty state has to hedge ("may not have been running, OR nothing happened") precisely because that data is not presented.
3. Four evidence treatments, and most bars in the default week view are likely too narrow to show any of them. Is the treatment set aimed at the wrong scale — should evidence be carried by the lane row rather than by each bar?
4. PRODUCT.md's primary job is "what did I do on this day", and there is no way to get to a day that is not today. Is the anti-calendar-grid commitment being read as "no date navigation" rather than "no date grid"?
