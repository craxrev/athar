<script lang="ts">
	import type { Block } from './archive';
	import Spans from './Spans.svelte';
	import Self from './Markdown.svelte';

	let { blocks }: { blocks: Block[] } = $props();

	/** Lines a code block shows before it is collapsed.
	 *
	 *  One archived turn on this machine is 25,600 characters — a single message
	 *  that would otherwise decide how long the whole conversation scrolls. The
	 *  collapsed state says how much it is holding, the way every other cap in the
	 *  app does. */
	const CODE_LINES = 24;
</script>

{#each blocks as block, i (i)}
	{#if block.b === 'p'}
		<p><Spans spans={block.spans} /></p>
	{:else if block.b === 'h'}
		<!-- Six markdown levels fold onto three steps of the existing ramp; a
		     conversation does not need its own type scale. -->
		<p class="h" data-level={Math.min(block.level, 3)}><Spans spans={block.spans} /></p>
	{:else if block.b === 'code'}
		{@const lines = block.text.split('\n')}
		{#if lines.length > CODE_LINES}
			<details class="code">
				<summary>
					<span class="lang">{block.lang ?? 'code'}</span>
					<span class="count">{lines.length} lines</span>
				</summary>
				<pre><code>{block.text}</code></pre>
			</details>
		{:else}
			<pre class="code"><code>{block.text}</code></pre>
		{/if}
	{:else if block.b === 'list'}
		{#if block.ordered}
			<ol>
				{#each block.items as item, j (j)}<li><Self blocks={item} /></li>{/each}
			</ol>
		{:else}
			<ul>
				{#each block.items as item, j (j)}<li><Self blocks={item} /></li>{/each}
			</ul>
		{/if}
	{:else if block.b === 'quote'}
		<blockquote><Self blocks={block.blocks} /></blockquote>
	{:else if block.b === 'table'}
		<!-- Its own scroller: a wide table must never make the page scroll. -->
		<div class="scroller">
			<table>
				{#if block.head.length}
					<thead>
						<tr>{#each block.head as cell, j (j)}<th><Spans spans={cell} /></th>{/each}</tr>
					</thead>
				{/if}
				<tbody>
					{#each block.rows as row, j (j)}
						<tr>{#each row as cell, k (k)}<td><Spans spans={cell} /></td>{/each}</tr>
					{/each}
				</tbody>
			</table>
		</div>
	{:else if block.b === 'rule'}
		<hr />
	{/if}
{/each}

<style>
	p {
		margin: 0 0 10px;
	}
	p:last-child {
		margin-bottom: 0;
	}

	.h {
		margin: 16px 0 8px;
		font-weight: 640;
		color: var(--text);
	}
	.h:first-child {
		margin-top: 0;
	}
	/* Body prose in the reader is 15px, so headings take the documented steps
	   above it: 19 and 15.5. A third level sets itself apart by weight rather than
	   by inventing a size between them. */
	.h[data-level='1'] {
		font-size: 19px;
	}
	.h[data-level='2'] {
		font-size: 15.5px;
	}
	.h[data-level='3'] {
		font-size: 15px;
		color: var(--text-dim);
	}

	ul,
	ol {
		margin: 0 0 10px;
		padding-left: 22px;
	}
	li {
		margin: 3px 0;
	}
	li::marker {
		color: var(--text-faint);
	}

	blockquote {
		margin: 0 0 10px;
		padding-left: 12px;
		border-left: 2px solid var(--line-strong);
		color: var(--text-dim);
	}

	pre.code,
	details.code {
		margin: 0 0 10px;
		border: 1px solid var(--line);
		border-radius: var(--radius-sm);
		background: var(--surface-inset);
	}
	pre {
		overflow-x: auto;
		padding: 10px 12px;
	}
	details.code pre {
		border-top: 1px solid var(--line);
	}
	code {
		font-family: var(--mono);
		font-size: 13px;
		line-height: 1.5;
		color: var(--text);
		white-space: pre;
	}

	summary {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 7px 12px;
		cursor: pointer;
		font-size: 13px;
		color: var(--text-dim);
	}
	.lang {
		font-family: var(--mono);
		color: var(--text);
	}
	.count {
		color: var(--text-faint);
	}

	.scroller {
		overflow-x: auto;
		margin: 0 0 10px;
	}
	table {
		border-collapse: collapse;
		font-size: 13.5px;
	}
	th,
	td {
		padding: 5px 10px;
		border: 1px solid var(--line);
		text-align: left;
		vertical-align: top;
	}
	th {
		font-weight: 620;
		color: var(--text);
		background: var(--surface);
	}

	hr {
		margin: 14px 0;
		border: none;
		border-top: 1px solid var(--line);
	}
</style>
