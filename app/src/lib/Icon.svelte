<script lang="ts">
	/** One drawn set, 20px grid, 1.75 stroke, round caps and joins. Every icon in
	 *  the app comes from here so weight and rhythm stay consistent. */
	type Name =
		| 'lanes'
		| 'stream'
		| 'session'
		| 'commit'
		| 'file'
		| 'clock'
		| 'warn'
		| 'witnessed'
		| 'inferred'
		| 'close'
		| 'chevron'
		| 'panelLeft'
		| 'panelRight'
		| 'search'
		| 'back'
		
		
		| 'settings';

	let { name, size = 20 }: { name: Name; size?: number } = $props();
</script>

<svg
	width={size}
	height={size}
	viewBox="0 0 20 20"
	fill="none"
	stroke="currentColor"
	stroke-width="1.75"
	stroke-linecap="round"
	stroke-linejoin="round"
	aria-hidden="true"
	focusable="false"
>
	{#if name === 'lanes'}
		<!-- Bars of differing length on stacked rows: the lanes view itself. -->
		<path d="M3 5.5h9M3 10h13M3 14.5h6" />
	{:else if name === 'stream'}
		<path d="M6.5 4.5h11M6.5 10h11M6.5 15.5h11" />
		<circle cx="3.2" cy="4.5" r="1.1" fill="currentColor" stroke="none" />
		<circle cx="3.2" cy="10" r="1.1" fill="currentColor" stroke="none" />
		<circle cx="3.2" cy="15.5" r="1.1" fill="currentColor" stroke="none" />
	{:else if name === 'session'}
		<path d="M3.5 5.25A1.75 1.75 0 0 1 5.25 3.5h9.5A1.75 1.75 0 0 1 16.5 5.25v6.5a1.75 1.75 0 0 1-1.75 1.75H8l-4.5 3.25z" />
	{:else if name === 'commit'}
		<!-- A node on a line: git's own notation. -->
		<circle cx="10" cy="10" r="3.25" />
		<path d="M10 3v3.75M10 13.25V17" />
	{:else if name === 'file'}
		<path d="M11.5 3.25H6.5A1.75 1.75 0 0 0 4.75 5v10A1.75 1.75 0 0 0 6.5 16.75h7A1.75 1.75 0 0 0 15.25 15V7z" />
		<path d="M11.5 3.25V7h3.75" />
	{:else if name === 'clock'}
		<circle cx="10" cy="10" r="6.75" />
		<path d="M10 6.25V10l2.75 2" />
	{:else if name === 'warn'}
		<path d="M10 3.4 17 15.6H3z" />
		<path d="M10 8v3.4" />
		<circle cx="10" cy="13.6" r="0.9" fill="currentColor" stroke="none" />
	{:else if name === 'witnessed'}
		<!-- A closed loop: the link was observed end to end. -->
		<circle cx="10" cy="10" r="6.75" />
		<path d="M6.75 10.25 9 12.5l4.25-4.75" />
	{:else if name === 'inferred'}
		<!-- The same loop, broken: the link was reasoned, not seen. -->
		<path d="M13.6 4.6a6.75 6.75 0 1 1-8.2 1.5" />
		<path d="M6.75 10.25 9 12.5l4.25-4.75" />
	{:else if name === 'close'}
		<path d="M5.5 5.5l9 9M14.5 5.5l-9 9" />
	{:else if name === 'chevron'}
		<!-- Right navigates, and the same glyph rotated down discloses; callers set
		     the rotation. One glyph at one weight keeps the set coherent. -->
		<path d="M7.75 4.75 13 10l-5.25 5.25" />
	{:else if name === 'panelLeft'}
		<rect x="3.25" y="3.75" width="13.5" height="12.5" rx="2" />
		<path d="M8.25 3.75v12.5" />
	{:else if name === 'panelRight'}
		<rect x="3.25" y="3.75" width="13.5" height="12.5" rx="2" />
		<path d="M11.75 3.75v12.5" />
	{:else if name === 'search'}
		<circle cx="8.75" cy="8.75" r="5" />
		<path d="M12.5 12.5 16.75 16.75" />
	{:else if name === 'back'}
		<path d="M12.25 4.75 7 10l5.25 5.25" />{:else if name === 'settings'}
		<!-- Sliders rather than a gear: this pane is values you set, not machinery. -->
		<path d="M3.5 6.25h4M11 6.25h5.5M3.5 13.75h6.5M13.5 13.75h3" />
		<circle cx="9.25" cy="6.25" r="2" />
		<circle cx="11.75" cy="13.75" r="2" />{/if}
</svg>

<style>
	svg {
		display: block;
		flex: none;
	}
</style>
