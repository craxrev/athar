<script lang="ts">
	import type { Span } from './archive';
	import Self from './Spans.svelte';

	let { spans }: { spans: Span[] } = $props();
</script>

{#each spans as span, i (i)}{#if span.s === 't'}{span.text}{:else if span.s === 'c'}<code
			>{span.text}</code
		>{:else if span.s === 'b'}<strong><Self spans={span.spans} /></strong>{:else if span.s === 'i'}<em
			><Self spans={span.spans} /></em
		>{:else if span.s === 'a'}<span class="link" title={span.href}
			><Self spans={span.spans} /></span
		>{/if}{/each}

<style>
	code {
		font-family: var(--mono);
		/* Mono sits slightly smaller than the prose around it, but never under the
		   floor: inside a 13.5px table cell, 0.92em alone computes to 12.42px. */
		font-size: max(13px, 0.92em);
		padding: 1px 5px;
		border-radius: 4px;
		background: var(--surface-inset);
		color: var(--text);
	}

	strong {
		font-weight: 640;
		color: var(--text);
	}

	/* Not an anchor: nothing in this window navigates, and an archive that opened a
	   browser from a five-year-old link would be doing something unasked. The
	   destination stays readable on the title, and the underline is dotted rather
	   than solid because a solid one reads as something you can click. */
	.link {
		color: var(--text);
		border-bottom: 1px dotted var(--line-strong);
	}
</style>
