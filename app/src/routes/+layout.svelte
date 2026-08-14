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
		/* One step *below* surface. The ramp otherwise only rises — raised, hover —
		   which left nothing to express a panel opened inside a row. A raised
		   surface lifts with a shadow; an inset one steps down the ramp. */
		--surface-inset: #0e1014;
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

		/* Category identity, defined once. Each hue needs three forms: the solid
		   for legend swatches, a translucent fill for lane bars and chips, and a
		   lightened tint for text sitting on that fill — the solid hue is not
		   legible as text on a dark ground. Categories come from the filesystem,
		   so this set is open-ended; an unknown category falls back to neutral. */
		--cat-work: #4c8dff;
		--cat-work-fill: rgba(76, 141, 255, 0.14);
		--cat-work-bar: rgba(76, 141, 255, 0.42);
		--cat-work-tint: #9dc0ff;

		--cat-research: #e93d97;
		--cat-research-fill: rgba(233, 61, 151, 0.16);
		--cat-research-bar: rgba(233, 61, 151, 0.38);
		--cat-research-tint: #f79ec9;

		--cat-personal: #56c98a;
		--cat-personal-fill: rgba(86, 201, 138, 0.14);
		--cat-personal-bar: rgba(86, 201, 138, 0.38);
		--cat-personal-tint: #8fdcb1;

		--cat-freelance: #f2a93b;
		--cat-freelance-fill: rgba(242, 169, 59, 0.14);
		--cat-freelance-bar: rgba(242, 169, 59, 0.38);
		--cat-freelance-tint: #f6c987;

		--radius: 9px;
		--radius-sm: 6px;

		/* Motion by purpose, not by feel.
		   `state` is the response to a pointer or a toggle. `live` is the one kind
		   that repeats: something is happening right now and will stop on its own.
		   Entrances keep their own curve where they are authored, one per surface. */
		--motion-state: 120ms ease-out;
		--motion-live: 1.6s ease-in-out;

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

	/* The one written button in the app: settings actions, and the recovery button
	   on an error state. Defined once here because a second copy drifts — the two
	   had already diverged on hover and disabled handling. */
	:global(.act) {
		padding: 6px 12px;
		border: 1px solid var(--line-strong);
		border-radius: var(--radius-sm);
		background: var(--surface-raised);
		color: var(--text);
		font-size: 14px;
		font-weight: 540;
		transition: background var(--motion-state), color var(--motion-state);
	}
	:global(.act:hover:not(:disabled)) {
		background: var(--surface-hover);
	}
	:global(.act:disabled) {
		opacity: 0.55;
		cursor: default;
	}
	/* The primary of a pair, and never more than one in a group. */
	:global(.act.strong) {
		background: var(--accent-soft);
		border-color: var(--accent-edge);
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
		border-radius: 999px;
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
