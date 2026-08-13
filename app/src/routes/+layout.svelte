<script lang="ts">
	let { children } = $props();
</script>

{@render children()}

<style>
	:global(:root) {
		/* Dark only, by decision rather than default: this is read at a desk on
		   the machine that produced the work, usually late. Colours are tokens so
		   a light mode later is a swap rather than a rewrite. */
		--ground: #0b0c0f;
		--surface: #12141a;
		--surface-raised: #191c24;
		--surface-hover: #1f2330;
		--line: rgba(255, 255, 255, 0.07);
		--line-strong: rgba(255, 255, 255, 0.14);

		--text: #edeef2;
		--text-dim: #a2a8b6;
		--text-faint: #878e9d;

		/* One accent, as flat fill for selection and live state — never as glow.
		   Magenta because git owns red and green semantically, and amber is
		   reserved for uncertainty. */
		--accent: #e93d97;
		--accent-soft: rgba(233, 61, 151, 0.16);
		--accent-edge: rgba(233, 61, 151, 0.42);
		--on-accent: #14040c;

		/* Uncertainty, and git's own vocabulary kept intact. */
		--amber: #f2a93b;
		--amber-soft: rgba(242, 169, 59, 0.14);
		--add: #56c98a;
		--del: #e5555c;

		--radius: 9px;
		--radius-sm: 6px;

		/* Elevation carries an offset and a soft blur. A raised surface that only
		   changes its border is not raised. */
		--lift-1: 0 1px 2px rgba(0, 0, 0, 0.4), 0 2px 8px rgba(0, 0, 0, 0.28);
		--lift-2: 0 2px 4px rgba(0, 0, 0, 0.45), 0 10px 26px rgba(0, 0, 0, 0.36);

		/* The type floor. Nothing in the interface sets smaller than --fs-min,
		   because "no small typography" is a binding constraint, and small type in
		   a light weight is the exact combination it rules out. */
		--fs-min: 13px;
		--fs-meta: 13.5px;

		/* Operate mode: a workhorse UI stack, set solid and generous. No hairline
		   weights and no micro labels anywhere. */
		--sans: -apple-system, BlinkMacSystemFont, 'Segoe UI', system-ui, sans-serif;
		--mono: ui-monospace, 'SF Mono', 'JetBrains Mono', Menlo, monospace;

		--titlebar: 30px;
	}

	:global(*),
	:global(*::before),
	:global(*::after) {
		box-sizing: border-box;
	}

	:global(html),
	:global(body) {
		height: 100%;
		margin: 0;
	}

	:global(body) {
		/* Transparent so the window's vibrancy material shows through the rail.
		   Panes paint their own ground. */
		background: transparent;
		color: var(--text);
		font-family: var(--sans);
		font-size: 15px;
		line-height: 1.5;
		-webkit-font-smoothing: antialiased;
		overflow: hidden;
	}

	:global(button) {
		font: inherit;
		color: inherit;
		background: none;
		border: none;
		cursor: pointer;
	}

	:global(:focus-visible) {
		outline: 2px solid var(--accent);
		outline-offset: 2px;
		border-radius: var(--radius-sm);
	}

	:global(::selection) {
		background: var(--accent-soft);
	}

	:global(::-webkit-scrollbar) {
		width: 11px;
		height: 11px;
	}
	:global(::-webkit-scrollbar-thumb) {
		background: rgba(255, 255, 255, 0.12);
		border: 3px solid transparent;
		background-clip: content-box;
		border-radius: 8px;
	}
	:global(::-webkit-scrollbar-thumb:hover) {
		background: rgba(255, 255, 255, 0.22);
		background-clip: content-box;
	}
	:global(::-webkit-scrollbar-track) {
		background: transparent;
	}

	:global(.num) {
		font-family: var(--mono);
		font-variant-numeric: tabular-nums;
		font-size: 0.92em;
	}
</style>
