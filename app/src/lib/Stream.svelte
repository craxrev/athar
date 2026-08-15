<script lang="ts">
	import Icon from './Icon.svelte';
	import type { BlockDetail, CommitSummary, SessionSummary } from './archive';
	import { clock, day, dayKey, duration, fullDay, tokens } from './format';
	import { clusterMoments, type Moment } from './moments';
	import { archive, type CommitFile } from './archive';

	let {
		blocks,
		selected,
		onSelect,
		onOpenSession
	}: {
		blocks: BlockDetail[];
		selected: number | null;
		onSelect: (blockId: number) => void;
		onOpenSession: (id: string) => void;
	} = $props();

	/** Derived once per block list rather than per render.
	 *
	 *  `entriesOf` sorts and clusters, and `composition` builds strings; both were
	 *  called straight from the template, so both ran for all 300 blocks every time
	 *  anything re-rendered — including selecting a block, which changes nothing
	 *  either of them reads. */
	let prepared = $derived.by(() => {
		const m = new Map<number, { entries: Entry[]; composition: string }>();
		for (const block of blocks) {
			m.set(block.id, { entries: entriesOf(block), composition: composition(block) });
		}
		return m;
	});

	/** Day headings are dividers in one continuous column, not a grid of dates. */
	let days = $derived.by(() => {
		const out: { key: string; at: number; blocks: BlockDetail[] }[] = [];
		for (const block of blocks) {
			const key = dayKey(block.started_ms);
			let group = out.find((g) => g.key === key);
			if (!group) {
				group = { key, at: block.started_ms, blocks: [] };
				out.push(group);
			}
			group.blocks.push(block);
		}
		return out;
	});

	type Entry = { at: number; key: string } & (
		| { kind: 'session'; session: SessionSummary }
		| { kind: 'commit'; commit: CommitSummary }
		| { kind: 'files'; moment: Moment }
	);

	/** A block's contents in the order they happened.
	 *
	 *  Grouping by kind put every file change below every commit regardless of when
	 *  either occurred, so a block announcing a time span read as three buckets. A
	 *  session is placed at its first record *inside* this block: a resumed
	 *  session's own start can be days earlier and would sort it to the top of a
	 *  block it merely overlaps. */
	function entriesOf(block: BlockDetail): Entry[] {
		const out: Entry[] = [];
		for (const session of block.sessions) {
			out.push({
				at:
					session.first_seen_ms ??
					Math.max(session.started_ms ?? block.started_ms, block.started_ms),
				key: `session\u0000${session.id}`,
				kind: 'session',
				session
			});
		}
		for (const commit of block.commits) {
			out.push({ at: commit.ts_ms, key: `commit\u0000${commit.sha}`, kind: 'commit', commit });
		}
		// Keyed on the moment's own timestamp, not its index. Clustering is 60s
		// wide so two moments cannot share one, and a rescan that inserts an
		// earlier moment no longer shifts every open row onto different data.
		for (const moment of clusterMoments(block.file_changes)) {
			out.push({ at: moment.at, key: `files\u0000${moment.at}`, kind: 'files', moment });
		}
		return out.sort((a, b) => a.at - b.at);
	}

	/** The buckets used to convey a block's shape by their size. With the contents
	 *  interleaved, the header carries it instead. */
	function composition(block: BlockDetail): string {
		const parts: string[] = [];
		const sessions = block.sessions.filter((s) => !s.continued).length;
		if (sessions) parts.push(`${sessions} session${sessions === 1 ? '' : 's'}`);
		if (block.commits.length)
			parts.push(`${block.commits.length} commit${block.commits.length === 1 ? '' : 's'}`);
		if (block.file_changes.length)
			parts.push(`${block.file_changes.length} file${block.file_changes.length === 1 ? '' : 's'}`);
		return parts.join(' · ');
	}

	const tierLabel: Record<string, string> = {
		certain: 'The transcript records the assistant running this commit',
		strong: "Inferred — the commit's files were written in this session",
		weak: 'Inferred from timing alone; likely committed by hand'
	};

	/** Files a commit touched, loaded on demand and kept once loaded.
	 *
	 *  From the archive, not from git: this answers for a repository that has since
	 *  been deleted and for commits git has already garbage-collected, which a live
	 *  diff cannot. */
	let expanded = $state<Record<string, CommitFile[] | 'loading' | 'error'>>({});
	/** Save-moments carry their files already, so disclosure is just open/closed. */
	let openMoments = $state<Record<string, boolean>>({});

	async function toggleFiles(sha: string) {
		if (expanded[sha]) {
			delete expanded[sha];
			return;
		}
		expanded[sha] = 'loading';
		try {
			expanded[sha] = await archive.commitFiles(sha);
		} catch {
			expanded[sha] = 'error';
		}
	}

	/** A resumed session can begin days before the block it continues into, so the
	 *  date shows whenever the clock alone would mislead. */
	function continuedFrom(startedMs: number | null, blockMs: number): string {
		if (startedMs === null) return 'earlier';
		const sameDay = new Date(startedMs).toDateString() === new Date(blockMs).toDateString();
		return sameDay ? clock(startedMs) : `${day(startedMs)} ${clock(startedMs)}`;
	}
</script>

<div class="stream">
	{#each days as d (d.key)}
		<div class="day">
			<h3>
				{fullDay(d.at)}
				<span class="num count">{d.blocks.length} block{d.blocks.length === 1 ? '' : 's'}</span>
			</h3>

			{#each d.blocks as block (block.id)}
				<article class:on={selected === block.id}>
					<button
						class="head"
						aria-pressed={selected === block.id}
						data-block={block.id}
						onclick={() => onSelect(block.id)}
					>
						<span class="num when">{clock(block.started_ms)}</span>
						<span class="project">{block.project}</span>
						<span class="swatch" data-category={block.category}>{block.category}</span>
						<span class="composition">{prepared.get(block.id)?.composition ?? ''}</span>
						<span class="num span">{duration(block.ended_ms - block.started_ms)}</span>
					</button>

					{#each prepared.get(block.id)?.entries ?? [] as entry (entry.key)}
						{#if entry.kind === 'session'}
							<button
								class="item session"
								class:continued={entry.session.continued}
								onclick={() => onOpenSession(entry.session.id)}
							>
								<span class="num at">{clock(entry.at)}</span>
								<Icon name="session" size={17} />
								<span class="body">
									<span class="title">{entry.session.title}</span>
									<span
										class="meta"
										title={entry.session.continued
											? 'This block is not where the session began, so its prompts, tools and tokens are counted once, at the block where it started.'
											: undefined}
									>
										{#if entry.session.continued}
											continues from {continuedFrom(entry.session.started_ms, block.started_ms)}
										{:else}
											<span class="num">{entry.session.prompts}</span> prompts ·
											<span class="num">{entry.session.tool_calls}</span> tools ·
											<span class="num">
												{tokens(entry.session.input_tokens + entry.session.output_tokens)}
											</span>
											tokens
											{#if entry.session.models.length}· {entry.session.models.join(', ')}{/if}
										{/if}
									</span>
								</span>
								{#if !entry.session.has_transcript}
									<span
										class="flag amber"
										title="Claude Code deleted this transcript; only the prompt history survives"
									>
										<Icon name="warn" size={14} /> prompts only
									</span>
								{/if}
								<span class="chev"><Icon name="chevron" size={16} /></span>
							</button>
						{:else if entry.kind === 'commit'}
							<button
								class="item commit"
								class:open={!!expanded[entry.commit.sha]}
								onclick={() => toggleFiles(entry.commit.sha)}
								aria-expanded={!!expanded[entry.commit.sha]}
							>
								<span class="num at">{clock(entry.at)}</span>
								<Icon name="commit" size={17} />
								<span class="body">
									<span class="title">{entry.commit.subject}</span>
									<span class="meta">
										<span class="num sha">{entry.commit.short}</span>
										<span class="num add">+{entry.commit.insertions}</span>
										<span class="num del">−{entry.commit.deletions}</span>
										{#if entry.commit.tier}
											<span
												class="tier"
												data-tier={entry.commit.tier}
												title={tierLabel[entry.commit.tier]}
											>
												<Icon
													name={entry.commit.tier === 'certain' ? 'witnessed' : 'inferred'}
													size={13}
												/>
												{entry.commit.tier === 'certain'
													? 'witnessed'
													: entry.commit.tier === 'strong'
														? 'files match'
														: 'inferred'}
											</span>
										{:else}
											<span class="tier" data-tier="none">unattributed</span>
										{/if}
									</span>
								</span>
								{#if entry.commit.unreachable}
									<span
										class="flag amber"
										title="No ref reaches this commit. Git will collect it; lore has already kept it."
									>
										<Icon name="warn" size={14} /> only in lore
									</span>
								{/if}
								<span class="chev disclose" class:turned={!!expanded[entry.commit.sha]}>
									<Icon name="chevron" size={16} />
								</span>
							</button>

							{#if expanded[entry.commit.sha]}
								{@const files = expanded[entry.commit.sha]}
								<div class="touched">
									{#if files === 'loading'}
										<p class="note">Reading the archive…</p>
									{:else if files === 'error'}
										<p class="note">This commit's file list could not be read.</p>
									{:else}
										<p class="inhead">In this commit</p>
										<ul>
											{#each files as f, i (i)}
												<li>
													<span class="fpath" title={f.path}>{f.path}</span>
													{#if f.added === null && f.deleted === null}
														<span class="binary">binary</span>
													{:else}
														<span class="num add">+{f.added ?? 0}</span>
														<span class="num del">−{f.deleted ?? 0}</span>
													{/if}
												</li>
											{/each}
										</ul>
										{#if entry.commit.file_count > files.length}
											<p class="note">
												{entry.commit.file_count - files.length} more files were in this
												commit than the archive kept.
											</p>
										{/if}
									{/if}
								</div>
							{/if}
						{:else}
							<button
								class="item files"
								class:open={openMoments[entry.key]}
								onclick={() => (openMoments[entry.key] = !openMoments[entry.key])}
								aria-expanded={!!openMoments[entry.key]}
							>
								<span class="num at">{clock(entry.at)}</span>
								<Icon name="file" size={17} />
								<span class="body">
									<span class="title">
										{entry.moment.files.length} file{entry.moment.files.length === 1 ? '' : 's'}
										saved, not committed
									</span>
									<span class="meta">
										{[...new Set(entry.moment.files.map((f) => f.state))].join(' · ')}
									</span>
								</span>
								<span class="chev disclose" class:turned={openMoments[entry.key]}>
									<Icon name="chevron" size={16} />
								</span>
							</button>

							{#if openMoments[entry.key]}
								<div class="touched">
									<p class="inhead">Saved, not committed</p>
									<ul>
										{#each entry.moment.files as f, i (i)}
											<li>
												<span class="fpath" title={f.path}>{f.path}</span>
												<span class="state" data-state={f.state}>{f.state}</span>
											</li>
										{/each}
									</ul>
								</div>
							{/if}
						{/if}
					{/each}

					{#if !block.sessions.length && !block.commits.length && !block.file_changes.length}
						<p class="bare">
							<span class="num">{block.records}</span> records archived here — harness state and
							prompt history, with nothing the timeline itemises.
						</p>
					{/if}
				</article>
			{/each}
		</div>
	{/each}
</div>

<style>
	/* The article around these buttons is overflow:hidden for its radius, which
	   clipped an outset focus ring on the left, right and top edges. Drawn inset
	   instead, so the primary selection targets in this view keep a visible ring. */
	.head:focus-visible,
	.item:focus-visible {
		outline: 2px solid var(--accent);
		outline-offset: -2px;
		border-radius: var(--radius-sm);
	}

	.stream {
		flex: 1;
		min-height: 0;
		overflow-y: auto;
		padding: 4px 20px 40px;
	}

	article {
		content-visibility: auto;
		contain-intrinsic-size: auto 120px;
	}

	.day {
		padding-top: 26px;
	}

	h3 {
		display: flex;
		align-items: baseline;
		gap: 10px;
		margin: 0 0 10px;
		font-size: 15px;
		font-weight: 640;
		color: var(--text);
	}
	h3 .count {
		color: var(--text-faint);
		font-weight: 500;
	}

	article {
		border: 1px solid var(--line);
		border-radius: var(--radius);
		background: var(--surface);
		margin-bottom: 10px;
		overflow: hidden;
	}
	article.on {
		border-color: var(--accent-edge);
		background: color-mix(in oklab, var(--accent) 6%, var(--surface));
		box-shadow: var(--lift-1);
	}

	.head {
		display: flex;
		align-items: center;
		gap: 11px;
		width: 100%;
		padding: 11px 14px;
		text-align: left;
	}
	.head:hover {
		background: var(--surface-hover);
	}

	.when {
		color: var(--text-dim);
		font-weight: 550;
	}

	.project {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		font-size: 15.5px;
		font-weight: 620;
		letter-spacing: -0.01em;
	}

	.swatch {
		font-size: var(--fs-meta);
		font-weight: 560;
		padding: 2px 7px;
		border-radius: var(--radius-pill);
		color: var(--text-dim);
		background: rgba(255, 255, 255, 0.06);
	}
	.swatch[data-category='work'] {
		color: var(--cat-work-tint);
		background: var(--cat-work-fill);
	}
	.swatch[data-category='research'] {
		color: var(--cat-research-tint);
		background: var(--cat-research-fill);
	}
	.swatch[data-category='personal'] {
		color: var(--cat-personal-tint);
		background: var(--cat-personal-fill);
	}
	.swatch[data-category='freelance'] {
		color: var(--cat-freelance-tint);
		background: var(--cat-freelance-fill);
	}

	/* What the buckets used to say by their size. */
	.composition {
		margin-left: auto;
		font-size: 13px;
		color: var(--text-faint);
		white-space: nowrap;
	}

	.span {
		color: var(--text-dim);
		font-weight: 550;
	}

	.item {
		display: flex;
		align-items: flex-start;
		gap: 10px;
		width: 100%;
		padding: 9px 14px;
		border-top: 1px solid var(--line);
		text-align: left;
		color: var(--text-dim);
	}
	.item :global(svg) {
		margin-top: 2px;
	}
	button.item:hover {
		background: var(--surface-hover);
	}
	.session :global(svg) {
		color: var(--accent);
	}
	.session.continued :global(svg) {
		color: var(--text-faint);
	}
	.session.continued .title {
		color: var(--text-dim);
		font-weight: 500;
	}

	/* The leading time is what makes the column read as a sequence, so it holds a
	   fixed width and every row aligns to it. */
	.at {
		flex: none;
		width: 42px;
		margin-top: 1px;
		color: var(--text-faint);
	}

	.body {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	.title {
		color: var(--text);
		font-size: 14.5px;
		font-weight: 540;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.meta {
		display: flex;
		align-items: center;
		flex-wrap: wrap;
		gap: 7px;
		font-size: 13px;
		color: var(--text-faint);
	}

	.sha {
		color: var(--text-dim);
	}
	.add {
		color: var(--add);
	}
	.del {
		color: var(--del);
	}

	.tier {
		display: inline-flex;
		align-items: center;
		gap: 4px;
		font-size: var(--fs-meta);
		font-weight: 540;
	}
	/* Witnessed and inferred are different claims and must not look alike. */
	.tier[data-tier='certain'] {
		color: var(--add);
	}
	.tier[data-tier='strong'] {
		color: var(--text-dim);
	}
	.tier[data-tier='weak'],
	.tier[data-tier='none'] {
		color: var(--amber);
	}

	.state {
		font-family: var(--sans);
		font-size: 13px;
		font-weight: 540;
		color: var(--text-faint);
	}
	.state[data-state='dirty'] {
		color: var(--amber);
	}

	.flag {
		display: inline-flex;
		align-items: center;
		gap: 4px;
		flex: none;
		font-size: var(--fs-meta);
		font-weight: 540;
		padding: 2px 8px;
		border-radius: var(--radius-pill);
	}
	.flag.amber {
		color: var(--amber);
		background: var(--amber-soft);
	}

	.chev {
		flex: none;
		color: var(--text-faint);
		margin-top: 2px;
	}
	button.item:hover .chev {
		color: var(--text);
	}

	/* The archived file list, opened in place. Indented to the same column as the
	   row's body so it reads as that commit's contents. */
	.touched {
		padding: 2px 14px 12px 66px;
		border-top: 1px solid var(--line);
		background: var(--surface-inset);
	}
	.inhead {
		margin: 0 0 6px;
		font-size: 13px;
		font-weight: 600;
		color: var(--text-faint);
	}

	.touched ul {
		margin: 0;
		padding: 0;
		list-style: none;
		display: flex;
		flex-direction: column;
		gap: 3px;
	}
	.touched li {
		display: flex;
		align-items: baseline;
		gap: 12px;
	}
	.touched .state {
		flex: none;
	}
	.fpath {
		flex: 1;
		min-width: 0;
		font-family: var(--mono);
		font-size: var(--fs-min);
		color: var(--text-dim);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.binary {
		font-size: 13px;
		color: var(--text-faint);
	}
	.note {
		margin: 6px 0 0;
		font-size: 13px;
		color: var(--text-faint);
	}

	/* Down at rest: this row opens in place rather than taking you elsewhere. */
	.chev.disclose {
		transform: rotate(90deg);
		transition: transform 140ms ease-out;
	}
	.chev.disclose.turned {
		transform: rotate(270deg);
		color: var(--text);
	}

	.bare {
		margin: 0;
		padding: 9px 14px;
		border-top: 1px solid var(--line);
		font-size: 13px;
		line-height: 1.45;
		color: var(--text-faint);
	}

	@media (prefers-reduced-motion: reduce) {
		.chev.disclose {
			transition: none;
		}
	}
</style>
