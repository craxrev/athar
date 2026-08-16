# athar

Answers "what did I actually do?" — for yesterday, last month, or years ago.

Your Claude Code conversations, git commits and file saves live in three places
that each forget. athar puts them on one timeline and keeps them.

macOS. Offline.

## What you get

- A timeline: a day by the hour, a week by the day, a month as tiles, all time as a heatmap
- Click anything to see its conversations, commits and files
- Read any conversation back in full
- Totals: hours, projects, sessions, commits, tokens, and how much the assistant wrote

## Why it keeps its own copy

- Claude Code deletes transcripts after 30 days
- git garbage-collects unreachable commits after about the same
- a file save leaves one modified time, overwritten by the next

## What it reads

| Source | From |
|---|---|
| Claude Code | `~/.claude/projects/**/*.jsonl`, `history.jsonl` |
| git | repos under the configured roots, including the reflog |
| files | modified times under the configured roots |

Conversations in full. Not source code — git has it. File counts are a floor:
two saves between scans leave one timestamp.

## AI attribution

| Tier | Basis |
|---|---|
| `witnessed` | the transcript shows the assistant running `git commit` |
| `files match` | the commit's files were written in that session |
| `inferred` | timing alone |
| `unattributed` | no session |

athar uses no model itself.

## Requirement

Scans only while open. Open it monthly, or that month goes with the sources.

## Build

```sh
cd app && npm install
npm run tauri dev
CI=true npm run tauri build   # CI=true skips a DMG step needing automation permission
```

## First run

1. Choose a root — the folder your projects live in
2. Scan

Claude Code is read without a root. Git and file changes need one.

## CLI

```sh
/Applications/athar.app/Contents/MacOS/athar-collector doctor
```

`scan` `stats` `doctor` `rebuild` `day` `config` `prune` `check`

`ATHAR_HOME=/tmp/test` for a throwaway profile.

## License

MIT — see [LICENSE](LICENSE).
