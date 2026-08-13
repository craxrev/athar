# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

Rendered inside a Tauri desktop window on macOS. The design language is web, not native macOS — this is not an `adaptive` product. Single window, no menu-bar item and no tray (confirmed).

## Stack

Confirmed by the user:

- **Frontend:** SvelteKit + TypeScript, static adapter (no SSR)
- **Shell:** Tauri 2 (stable line 2.11.x)
- **Core + collectors:** Rust
- **Storage:** SQLite (WAL), with full-text search over archived text

Process split: a background collector process is the sole DB writer; the Tauri app opens the database read-only. The app has no tray or menu-bar surface, so collector health must be visible **inside the window**.

## Users

A single working developer — the author — reviewing their own past work on their own machine.

Two situations, both confirmed as primary:

1. **Recall.** Reconstructing what happened on a given day, week, or month, after the fact, when memory has faded and the artifacts are scattered across three systems.
2. **Insight.** Understanding their own working patterns over longer spans — where hours actually go, which projects consume them, how much of the work is AI-assisted.

Built as a personal tool now, but with the explicit intent that other developers could use it later. Consequence: no hardcoded machine-specific assumptions; scanned roots, scan interval, and excluded paths are configuration, not constants.

## Product Purpose

Developer work leaves evidence across systems that were never designed to be read together, and that actively destroy their own history:

- Claude Code deletes session transcripts after 30 days by default
- Git garbage-collects unreachable commits (deleted branches, pre-rebase history) after roughly 30 days
- Ordinary file saves leave nothing behind but a modified timestamp that the next save overwrites

lore reads all three on a schedule and keeps what it finds in a database it owns. The sources are transient; lore's copy is permanent.

Success is being able to answer "what did I do on this day" — and "how has my work changed over these months" — from a record that still exists long after every original source has erased it.

## Positioning

The mechanism is **archival, not observation.** lore does not sit in the loop: no hooks installed in the user's harnesses, no live filesystem watcher, no daemon that must be running at the moment work happens. It reads artifacts that already exist on disk, on a schedule, and copies them somewhere permanent before their owners delete them.

This produces two properties a live-observation tracker cannot have:

- Work done while lore was not running is still captured, because the evidence outlives the moment
- Retention is unbounded, because the archive is not subject to the sources' cleanup policies

The one requirement this creates: lore must run at least once inside each source's retention window (~30 days) or that window's history is lost for good.

## Operating Context

The user works under a single configured project root (52 GB, ~307K files excluding `node_modules`, `.git`, `target`, `build`), primarily in Claude Code, alongside manual editing and manual git use.

Measured across the three project groups: **13 git repositories at project level, and 43 project directories with no git at all** (49 `.git` directories exist in total once nested ones are counted). The majority of the user's research work is not version-controlled. This is a defining fact about the product: for most projects, filesystem evidence and Claude transcripts are the *only* record that exists.

Reviewing happens after the fact, at a desk, on the same machine that produced the work — not during the work itself. lore is opened deliberately to look something up or take stock, not left open as a monitor.

## Capabilities and Constraints

**Sources ingested (v1):**

| Source | Read from | Timestamps |
|---|---|---|
| Claude Code sessions | `~/.claude/projects/**/*.jsonl`, `~/.claude/history.jsonl`, `~/.claude/file-history/` | Exact, in the data |
| Git | Repositories under the configured roots, including `.git/logs/HEAD` reflog | Exact, in the data |
| File changes | Filesystem modified-times under the configured roots | Exact per change; incomplete between scans |

**Capture model:** scheduled scan, configurable interval, hourly by default. No live watching, no hooks. Each scan resumes from a stored per-file byte offset, so append-only transcripts are never re-parsed. Re-scanning is idempotent.

**File-change detection rule.** Whether a file is scanned depends on its *git state*, not on whether its project uses git:

- **Inside a git repository** — only files git cannot already account for: **dirty or untracked**, from `git status`. Files matching HEAD are git's job and are ignored. This requires no filesystem walk of tracked trees, and it eliminates mtime false positives for free: `git checkout`, rebase, `npm install` and build steps rewrite modified-times but leave files **clean**, so they are excluded by definition rather than by heuristic.
- **Outside a git repository** — mtime is the only evidence in existence, so the tree is walked in full, minus ignore rules. This path covers the 43 non-git project directories and therefore most of the user's research history.

The value of the in-repo case is uncommitted work: an afternoon of editing that was never committed is invisible to git and visible only here.

**What is stored:** conversations in full (both sides — user prompts and AI replies), tool calls with their file paths and error status, per-message token usage and model, commit metadata (sha, time, message, branch, file names, +/− counts), and file-change timestamps.

**What is deliberately not stored:** source code. Git already stores it more efficiently — measured on the user's own repos, printing every patch as text came to 5.9 MB for a 194-commit repo and 221 MB for a 2,182-commit repo, against 133 KB and 2.4 MB of metadata respectively. Diffs are fetched live from git when the user opens a commit. Large tool outputs (file dumps, grep results) are truncated on archive; they are the bulk of transcript volume and are not read.

**Correlation lore derives:**

- Activity blocks — contiguous work clustered by project and idle gap
- Session → commit attribution. Commits made by the AI are **certain**, because `git commit` appears as a recorded Bash tool call in the transcript. Other commits in the same repo and time block are attributed to the user.
- File provenance — which conversation or commit touched a file, and whether the edit was AI-made or human-made
- AI-assisted ratio

**Known limits, which the interface must not paper over:**

- File-change **timestamps are exact** (the mtime is the real time of the last save); what is incomplete is **coverage**. Three saves inside one scan interval leave one mtime, so any per-file change count is a floor, never a total. Interfaces displaying such counts must present them as minimums.
- File changes carry no content, only that a change occurred and when
- Because most projects are not in git, a missed scan is genuinely lost work for those projects, where commits would otherwise have carried exact history. This is the strongest argument for a short scan interval.
- Commands the user ran in their own terminal are invisible; only harness-run commands are captured
- Anything older than the sources' retention that lore never scanned in time does not exist and is not recoverable
- Shell history was considered as a source and **rejected** by the user

**Architecture constraint:** archived raw records are immutable and append-only; every derived table is rebuildable from them. Collector adapters are pluggable, but Claude Code is the only implementation in v1 (Codex and opencode data exist on the machine and are deliberately out of scope).

## Brand Commitments

Working name **lore**, inferred from the project directory `lore-app` — *not confirmed by the user*.

**Standing visual preference, confirmed after two direction re-rolls:** no metaphor world. A clean, modern, dark product UI executed impeccably. The user reviewed and rejected roughly twenty-six metaphor-driven worlds; the conventional path is the deliberate commitment, not a fallback.

Binding constraints from the user:

- Dark, minimal, futuristic. **Dark only** — light mode is explicitly deferred and uninteresting for now, though colours are expressed as tokens so it stays a swap rather than a rewrite.
- Glass: real macOS window vibrancy on the sidebar, not a CSS blur imitation. Dense content sits on solid dark.
- **No skinny or small typography** — solid weights, generous sizes, no hairline rules, no micro-labels
- One committed accent color against dark neutrals
- Easy on the eyes, easy to digest, straight to the point, never cumbersome
- Dev-friendly: get what you want without friction or frustration
- Ambition belongs in UX and interaction, not in ornament

**Craft bar: Raycast.** Named by the user. The bar is its commitment to a single accent and its near-total absence of chrome, with everything reachable by keyboard — not its specific hue, which stays Raycast's.

## Evidence on Hand

All figures below were measured on the user's machine during planning and are real:

- `~/.claude/projects` — 219 MB, 156 transcript files, oldest surviving mtime 2026-07-09 (the 30-day cleanup window in action)
- `~/.claude/history.jsonl` — 9,838 prompts spanning 2026-01-23 to 2026-08-13, i.e. roughly six months of prompt history whose full transcripts have already been deleted
- `~/.claude/file-history` — 16 MB of real before/after file backups from AI edits
- Transcript composition, measured on one 948 KB session: user prompt text 1 KB, assistant text 12 KB, tool-call inputs 64 KB, thinking 159 KB, tool results 163 KB. Signal is a small fraction of volume.
- The configured project root — 52 GB, 1.6 M files, 307 K after excluding build and dependency directories; a full walk takes seconds
- Project-level version control: **13 git repositories, 43 non-git project directories** across three project groups (research, personal, work)

No users besides the author, no external validation, no benchmarks, no pricing, no deployment. Future work must not fabricate any.

Reference product: **Scribe** (getscribe.ai), unreleased, private beta — captures commits, file saves, and AI sessions into an auto-billing timeline. lore shares its capture premise but deliberately excludes invoicing and billing.

## Product Principles

1. **Archive, don't mirror.** The sources delete themselves. lore's copy is the durable record, and the design must never imply that history begins where the sources' retention begins.
2. **Never duplicate what another system stores better.** Metadata lives in lore; content is fetched live from the system that already holds it.
3. **Evidence over estimation.** Every entry traces to a real artifact on disk. Time is derived from timestamps that actually exist — nothing is invented to fill a gap.
4. **State the gaps.** Coverage is uneven by design (exact for git and transcripts, coarse for file saves, absent for terminal work). The interface must make the difference legible rather than presenting a smooth, uniformly confident record.
5. **Everything derived is rebuildable.** Raw records are immutable; interpretations can always be recomputed as adapters improve.
