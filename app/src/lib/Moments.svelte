<script lang="ts">
	import type { FileChangeSummary } from './archive';
	import { clock, duration, shortPath } from './format';
	import { clusterMoments, GAP_MS } from './moments';

	let {
		changes,
		limit = 0
	}: { changes: FileChangeSummary[]; limit?: number } = $props();

	let moments = $derived(clusterMoments(changes));

	let shown = $derived(limit > 0 ? moments.slice(0, limit) : moments);
	let hiddenFiles = $derived(
		moments.slice(shown.length).reduce((n, m) => n + m.files.length, 0)
	);
</script>

<ol class="moments">
	{#each shown as moment (moment.at)}
		{#if moment.gapFromPrevious >= GAP_MS}
			<!-- Not aria-hidden: an idle stretch is part of the record. It says the
			     work stopped, which is exactly the kind of gap this product exists to
			     keep. Only the drawn rule beside it is decorative. -->
			<li class="gap">
				<span class="rule" aria-hidden="true"></span>
				<span class="elapsed">{duration(moment.gapFromPrevious)} later</span>
			</li>
		{/if}
		<li class="moment">
			<span class="spine" aria-hidden="true"><i class="mark"></i></span>
			<span class="body">
				<span class="head">
					<span class="num at">{clock(moment.at)}</span>
					<span class="count">
						{moment.files.length} file{moment.files.length === 1 ? '' : 's'}
					</span>
				</span>
				<span class="files">
					{#each moment.files as f, i (i)}
						<span class="file">
							<span class="path" title={f.path}>{shortPath(f.path, 2)}</span>
							<span class="state" data-state={f.state}>{f.state}</span>
						</span>
					{/each}
				</span>
			</span>
		</li>
	{/each}

	{#if hiddenFiles > 0}
		<li class="moment more">
			<span class="spine" aria-hidden="true"><i class="mark faint"></i></span>
			<span class="body">
				<span class="count">
					+{hiddenFiles} more change{hiddenFiles === 1 ? '' : 's'} in
					{moments.length - shown.length} later moment{moments.length - shown.length === 1 ? '' : 's'}
				</span>
			</span>
		</li>
	{/if}
</ol>

<style>
	.moments {
		margin: 0;
		padding: 0;
		list-style: none;
	}

	.moment {
		display: flex;
		gap: 10px;
	}

	/* The spine is the timeline: a continuous rule through every moment, with a
	   mark where each one sits. */
	.spine {
		position: relative;
		flex: none;
		width: 9px;
		align-self: stretch;
	}
	.spine::before {
		content: '';
		position: absolute;
		left: 4px;
		top: 0;
		bottom: 0;
		width: 1px;
		background: var(--line-strong);
	}
	.moment:first-child .spine::before {
		top: 7px;
	}
	.moment:last-child .spine::before {
		bottom: auto;
		height: 7px;
	}

	.mark {
		position: absolute;
		left: 0;
		top: 4px;
		width: 9px;
		height: 9px;
		border-radius: var(--radius-circle);
		background: var(--text-dim);
		box-shadow: 0 0 0 3px var(--surface);
	}
	.mark.faint {
		background: var(--text-faint);
	}

	.body {
		flex: 1;
		min-width: 0;
		padding-bottom: 12px;
	}

	.head {
		display: flex;
		align-items: baseline;
		gap: 9px;
	}

	.at {
		color: var(--text);
		font-weight: 620;
	}

	.count {
		font-size: 13px;
		color: var(--text-faint);
	}

	.files {
		display: flex;
		flex-direction: column;
		gap: 2px;
		margin-top: 3px;
	}

	.file {
		display: flex;
		align-items: baseline;
		justify-content: space-between;
		gap: 10px;
		min-width: 0;
	}

	.path {
		flex: 1;
		min-width: 0;
		font-family: var(--mono);
		font-size: var(--fs-min);
		color: var(--text-dim);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.state {
		flex: none;
		font-size: 13px;
		font-weight: 540;
		color: var(--text-faint);
	}
	.state[data-state='dirty'] {
		color: var(--amber);
	}
	.state[data-state='untracked'] {
		color: var(--text-dim);
	}

	/* An idle stretch is part of the record: it says the work stopped. */
	.gap {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 2px 0 6px;
	}
	.rule {
		flex: none;
		width: 9px;
		align-self: stretch;
		position: relative;
	}
	.rule::before {
		content: '';
		position: absolute;
		left: 4px;
		top: -6px;
		bottom: -6px;
		width: 1px;
		background: repeating-linear-gradient(
			to bottom,
			var(--line-strong) 0 2px,
			transparent 2px 5px
		);
	}
	.elapsed {
		font-size: 13px;
		color: var(--text-faint);
	}
</style>
