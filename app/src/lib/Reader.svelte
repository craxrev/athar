<script lang="ts">
	import Icon from './Icon.svelte';
	import Markdown from './Markdown.svelte';
	import type { SessionDetail } from './archive';
	import { clock, clockRange, duration, fullDay, shortPath, tokens } from './format';

	let {
		detail,
		onClose
	}: { detail: SessionDetail; onClose: () => void } = $props();

	const s = $derived(detail.session);
	const span = $derived(
		s.started_ms !== null && s.ended_ms !== null ? s.ended_ms - s.started_ms : null
	);

	/** This surface replaced the timeline, so whatever had focus is gone. Taking
	 *  it here means a screen reader announces the region just entered rather
	 *  than falling silently to the document body. */
	let surface = $state<HTMLElement | null>(null);
	$effect(() => surface?.focus({ preventScroll: true }));
</script>

<!-- Reading takes over the window rather than opening a modal: a 200-message
     conversation needs the measure, and a modal would block everything else for
     a task that needs neither interruption nor protected focus. -->
<section
	class="reader"
	bind:this={surface}
	tabindex="-1"
	aria-label="Conversation reader"
>
	<div class="bar" data-tauri-drag-region>
		<button class="back" onclick={onClose}>
			<Icon name="back" size={18} />
			<span>Timeline</span>
		</button>
		<span class="crumb">
			{detail.project}
			<span class="dim">· {detail.category}</span>
		</span>
		<button class="close" onclick={onClose} aria-label="Close reader (Escape)">
			<Icon name="close" size={17} />
		</button>
	</div>

	<div class="scroll">
		<header>
			<h2 class="title">{s.title}</h2>
			<p class="facts">
				<span>{fullDay(s.started_ms)}</span>
				<span class="num">{clockRange(s.started_ms, s.ended_ms)}</span>
				{#if span !== null}
					<span
						class="num strong"
						title="First to last record, including idle time. The timeline's blocks are the time worked."
					>
						{duration(span)} span
					</span>
				{/if}
				<span><b class="num">{s.prompts}</b> prompts</span>
				<span><b class="num">{s.tool_calls}</b> tool calls</span>
				<span><b class="num">{tokens(s.input_tokens + s.output_tokens)}</b> tokens</span>
				{#if s.models.length}<span>{s.models.join(', ')}</span>{/if}
			</p>

			{#if !s.has_transcript}
				<p class="banner">
					<Icon name="warn" size={16} />
					<span>
						Claude Code deleted this transcript. Only the prompts athar archived remain;
						the replies are gone.
					</span>
				</p>
			{/if}

			{#if detail.commits.length || detail.files.length}
				<div class="outcome">
					{#if detail.commits.length}
						<div class="col">
							<h3>Produced</h3>
							{#each detail.commits as c (c.sha)}
								<div class="commit">
									<span class="subject">{c.subject}</span>
									<span class="figures">
										<span class="num sha">{c.short}</span>
										<span class="num add">+{c.insertions}</span>
										<span class="num del">−{c.deletions}</span>
										<span class="tier" data-tier={c.tier ?? 'none'}>
											{c.tier === 'certain' ? 'witnessed' : c.tier === 'strong' ? 'files match' : 'inferred'}
										</span>
									</span>
								</div>
							{/each}
						</div>
					{/if}
					{#if detail.files.length}
						<div class="col">
							<h3>Touched</h3>
							<ul>
								{#each detail.files.slice(0, 10) as f (f.path)}
									<li>
										<span class="path" title={f.path}>{shortPath(f.path, 2)}</span>
										{#if f.writes > 0}<span class="badge wrote">wrote</span>{:else}<span class="badge read">read</span>{/if}
									</li>
								{/each}
								{#if detail.files.length > 10}
									<li class="dim">+{detail.files.length - 10} more</li>
								{/if}
							</ul>
						</div>
					{/if}
				</div>
			{/if}
		</header>

		<div class="turns">
			{#each detail.turns as turn, i (i)}
				<article class={turn.role}>
					<div class="who">
						<span class="role">{turn.role === 'user' ? 'You' : 'Assistant'}</span>
						<span class="num at">{clock(turn.ts_ms)}</span>
					</div>
					{#if turn.blocks.length}
						<div class="text">
							<Markdown blocks={turn.blocks} />
							{#if turn.truncated}<p class="cut">shortened on archive</p>{/if}
						</div>
					{/if}
					{#if turn.tools.length}
						<ul class="tools">
							{#each turn.tools as t, j (j)}
								<li class:failed={t.failed}>
									<span class="tool">{t.name}</span>
									{#if t.target}<span class="target">{shortPath(t.target, 2)}</span>{/if}
									{#if t.failed}<span class="badge err">failed</span>{/if}
								</li>
							{/each}
						</ul>
					{/if}
				</article>
			{/each}

			{#if detail.turns.length === 0}
				<p class="none">Nothing of this conversation survives.</p>
			{/if}
		</div>
	</div>
</section>

<style>
	.reader:focus {
		outline: none;
	}

	.reader {
		display: flex;
		flex-direction: column;
		flex: 1;
		min-width: 0;
		min-height: 0;
		background: var(--ground);
		/* Content is visible from the first frame and settles up; the reader
		   replacing the whole window should read as arriving, not as a cut. */
		animation: arrive 260ms cubic-bezier(0.16, 1, 0.3, 1) both;
	}

	@keyframes arrive {
		from {
			opacity: 0.6;
			transform: translateY(6px);
		}
		to {
			opacity: 1;
			transform: none;
		}
	}

	@media (prefers-reduced-motion: reduce) {
		.reader {
			animation: none;
		}
	}

	.bar {
		display: flex;
		align-items: center;
		gap: 14px;
		flex: none;
		height: calc(var(--titlebar) + 18px);
		padding: 12px 18px 0;
		border-bottom: 1px solid var(--line);
	}

	.back {
		display: flex;
		align-items: center;
		gap: 5px;
		padding: 5px 10px 5px 6px;
		margin-left: var(--traffic-inset);
		border-radius: var(--radius-sm);
		color: var(--text-dim);
		font-size: 14px;
		font-weight: 540;
	}
	.back:hover {
		background: var(--surface-hover);
		color: var(--text);
	}

	.crumb {
		flex: 1;
		font-size: 14px;
		font-weight: 560;
		color: var(--text-dim);
	}
	.dim {
		color: var(--text-faint);
		font-weight: 500;
	}

	.close {
		padding: 6px;
		border-radius: var(--radius-sm);
		color: var(--text-faint);
	}
	.close:hover {
		background: var(--surface-hover);
		color: var(--text);
	}

	.scroll {
		flex: 1;
		overflow-y: auto;
		min-height: 0;
		padding: 0 24px 64px;
	}

	header {
		/* Reading measure: prose stays inside 70ch even in a wide window. */
		max-width: 78ch;
		margin: 0 auto;
		padding: 28px 0 8px;
	}

	.title {
		margin: 0;
		font-size: 30px;
		font-weight: 680;
		letter-spacing: -0.024em;
		line-height: 1.2;
		text-wrap: balance;
	}

	.facts {
		display: flex;
		flex-wrap: wrap;
		gap: 6px 16px;
		margin: 12px 0 0;
		font-size: 13.5px;
		color: var(--text-faint);
	}
	.facts b {
		color: var(--text-dim);
		font-weight: 600;
	}
	.facts .strong {
		color: var(--text);
		font-weight: 620;
	}

	.banner {
		display: flex;
		align-items: flex-start;
		gap: 9px;
		margin: 18px 0 0;
		padding: 11px 13px;
		border-radius: var(--radius-sm);
		background: var(--amber-soft);
		color: var(--amber);
		font-size: 13.5px;
		line-height: 1.5;
	}
	.banner :global(svg) {
		margin-top: 1px;
		flex: none;
	}

	.outcome {
		display: flex;
		flex-wrap: wrap;
		gap: 26px;
		margin-top: 24px;
		padding-top: 18px;
		border-top: 1px solid var(--line);
	}
	.col {
		flex: 1 1 260px;
		min-width: 0;
	}

	/* One level below the session title, and marked as one. These were `h2` beside
	   a 30px `h2` — three peers to a screen reader where the eye reads two levels
	   apart, with no way to navigate from the title into the columns. */
	h3 {
		margin: 0 0 8px;
		font-size: var(--fs-meta);
		font-weight: 640;
		color: var(--text-faint);
	}

	.commit {
		display: flex;
		flex-direction: column;
		gap: 3px;
		padding: 8px 0;
		border-bottom: 1px solid var(--line);
	}
	.commit:last-child {
		border-bottom: none;
	}
	.commit .figures {
		display: flex;
		flex-wrap: wrap;
		align-items: baseline;
		gap: 10px;
		font-size: 13px;
	}
	.sha {
		color: var(--text-faint);
	}
	.subject {
		color: var(--text);
		font-size: 14px;
		font-weight: 540;
		line-height: 1.4;
	}
	.add {
		color: var(--add);
	}
	.del {
		color: var(--del);
	}
	.tier {
		font-size: var(--fs-meta);
		font-weight: 540;
		color: var(--text-dim);
	}
	.tier[data-tier='certain'] {
		color: var(--add);
	}
	.tier[data-tier='weak'],
	.tier[data-tier='none'] {
		color: var(--amber);
	}

	.col ul {
		margin: 0;
		padding: 0;
		list-style: none;
	}
	.col li {
		display: flex;
		align-items: baseline;
		gap: 8px;
		padding: 2px 0;
		font-size: 13px;
	}
	.path {
		font-family: var(--mono);
		font-size: var(--fs-min);
		color: var(--text-dim);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.badge {
		flex: none;
		font-size: var(--fs-min);
		font-weight: 580;
		padding: 1px 6px;
		border-radius: var(--radius-pill);
		background: var(--fill-subtle);
		color: var(--text-faint);
	}
	/* Wrote and read is what the session did to the file, which is git's own
	   vocabulary rather than the accent's: green is content added, here as
	   everywhere else. The accent had it, and the accent is selection. */
	.badge.wrote {
		color: var(--add);
	}
	.badge.err {
		color: var(--del);
		background: var(--del-soft);
	}

	.turns {
		max-width: 78ch;
		margin: 0 auto;
		padding-top: 30px;
	}

	article {
		padding: 14px 0 16px;
		border-top: 1px solid var(--line);
	}

	.who {
		display: flex;
		align-items: baseline;
		gap: 9px;
		margin-bottom: 6px;
	}
	.role {
		font-size: 13px;
		font-weight: 640;
		color: var(--text-dim);
	}
	/* Your turns lead, the assistant's follow, and the ink ramp says so — the
	   accent is selection and live state, and a two-hundred-turn transcript would
	   have spent it a hundred times on neither. */
	article.user .role {
		color: var(--text);
	}
	.at {
		color: var(--text-faint);
		font-size: var(--fs-min);
	}

	/* The one reading surface, and the only place the 500 floor lifts: 500 across a
	   two-hundred-message transcript reads heavier than a long sitting wants.
	   Covers the markdown rendered inside it. */
	.text {
		overflow-wrap: anywhere;
		font-size: 15px;
		font-weight: 400;
		line-height: 1.62;
		color: var(--text);
	}

	/* A long conversation is two thousand turns, and markdown turns each one into
	   many more nodes than the plain text it replaced. This lets the browser skip
	   layout and paint for turns that are off screen, while leaving them in the
	   document — so find-in-page and the scrollbar still see the whole thing,
	   which a windowed list would break. The estimate keeps the scrollbar from
	   lurching as real heights replace it. */
	article {
		content-visibility: auto;
		contain-intrinsic-size: auto 180px;
	}
	article.assistant .text {
		color: var(--text-dim);
	}
	.cut {
		color: var(--amber);
		font-size: 13px;
	}

	.tools {
		margin: 10px 0 0;
		padding: 0;
		list-style: none;
		display: flex;
		flex-direction: column;
		gap: 3px;
	}
	.tools li {
		display: flex;
		align-items: baseline;
		gap: 8px;
		padding: 3px 9px;
		border-radius: var(--radius-sm);
		background: var(--surface);
		font-size: var(--fs-meta);
	}
	.tools li.failed {
		background: color-mix(in srgb, var(--del) 8%, transparent);
	}
	.tool {
		font-weight: 620;
		color: var(--text-dim);
		flex: none;
	}
	.target {
		font-family: var(--mono);
		font-size: var(--fs-min);
		color: var(--text-faint);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.none {
		color: var(--text-faint);
		font-size: 14px;
	}
</style>
