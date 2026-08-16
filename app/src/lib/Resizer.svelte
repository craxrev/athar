<script lang="ts">
	/** The grab edge between two panes.
	 *
	 *  A real `separator` rather than a styled div: this is the ARIA window-splitter
	 *  pattern, so it is focusable, carries its current width and its limits, and
	 *  answers the arrow keys. A pane you can only size with a mouse is a pane half
	 *  this product's users cannot size — the whole window is otherwise reachable
	 *  from the keyboard.
	 *
	 *  It takes no layout width at all. The hit zone and the lit line are pseudo
	 *  elements straddling the pane border that is already drawn there, so adding
	 *  the handle moved nothing. */
	let {
		value,
		min,
		max,
		base,
		label,
		/** Which side of this edge the pane being sized sits on. Dragging right
		 *  grows a pane on the left and shrinks one on the right. */
		side,
		onChange,
		onActive
	}: {
		value: number;
		min: number;
		max: number;
		/** The width a double-click, Enter or Space returns to. */
		base: number;
		label: string;
		side: 'left' | 'right';
		onChange: (next: number) => void;
		onActive: (dragging: boolean) => void;
	} = $props();

	let dragging = $state(false);

	const clamp = (n: number) => Math.max(min, Math.min(max, Math.round(n)));

	function onPointerDown(event: PointerEvent) {
		// Primary button only; a right-click on an edge is not a drag.
		if (event.button !== 0) return;
		const el = event.currentTarget as HTMLElement;
		const startX = event.clientX;
		const startValue = value;
		// Captured, so a fast drag that outruns the pointer keeps sizing instead of
		// stopping the moment the cursor leaves a 9px strip.
		el.setPointerCapture(event.pointerId);
		dragging = true;
		onActive(true);

		const move = (e: PointerEvent) => {
			const delta = (e.clientX - startX) * (side === 'left' ? 1 : -1);
			onChange(clamp(startValue + delta));
		};
		const stop = (e: PointerEvent) => {
			dragging = false;
			onActive(false);
			el.releasePointerCapture(e.pointerId);
			el.removeEventListener('pointermove', move);
			el.removeEventListener('pointerup', stop);
			el.removeEventListener('pointercancel', stop);
		};
		el.addEventListener('pointermove', move);
		el.addEventListener('pointerup', stop);
		el.addEventListener('pointercancel', stop);
	}

	/** The separator owns the arrow keys while it holds focus.
	 *
	 *  `stopPropagation` is load-bearing: the window handler binds ← and → to
	 *  stepping the range, and without this, sizing a pane would walk the timeline
	 *  a week at a time underneath it. */
	function onKeyDown(event: KeyboardEvent) {
		const step = event.shiftKey ? 64 : 16;
		const grow = side === 'left' ? 'ArrowRight' : 'ArrowLeft';
		const shrink = side === 'left' ? 'ArrowLeft' : 'ArrowRight';
		let next: number | null = null;
		if (event.key === grow) next = value + step;
		else if (event.key === shrink) next = value - step;
		else if (event.key === 'Home') next = min;
		else if (event.key === 'End') next = max;
		else if (event.key === 'Enter' || event.key === ' ') next = base;
		if (next === null) return;
		event.preventDefault();
		event.stopPropagation();
		onChange(clamp(next));
	}
</script>

<!-- A focusable separator is the ARIA window-splitter widget, and a widget is
     exactly what this is: it has a value, limits, and keys that change it. The
     linter's rule assumes `separator` is always the decorative kind. -->
<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div
	class="resizer"
	class:dragging
	role="separator"
	aria-orientation="vertical"
	aria-label={label}
	aria-valuenow={value}
	aria-valuemin={min}
	aria-valuemax={max}
	title="Drag to resize · double-click to reset"
	tabindex="0"
	onpointerdown={onPointerDown}
	onkeydown={onKeyDown}
	ondblclick={() => onChange(base)}
></div>

<style>
	/* Zero width on purpose: the panes already draw a 1px divider here, and a
	   handle that took space would have moved every pane the day it was added. */
	.resizer {
		position: relative;
		flex: none;
		width: 0;
		z-index: 2;
		cursor: col-resize;
	}
	/* The grab zone, straddling the divider. Nine pixels is what a pointer can
	   land on without aiming; the divider it sits over stays one. */
	.resizer::before {
		content: '';
		position: absolute;
		inset-block: 0;
		left: -4px;
		width: 9px;
	}
	/* The divider, lit. Two pixels rather than one so the live edge reads as a
	   state and not as a rendering artefact of the border underneath it. */
	.resizer::after {
		content: '';
		position: absolute;
		inset-block: 0;
		left: -1px;
		width: 2px;
		background: transparent;
		transition: background var(--motion-state);
	}
	.resizer:hover::after,
	.resizer.dragging::after {
		background: var(--accent);
	}
	/* The ring would be a 2px rounded box around a zero-width element, which draws
	   as a dot. The lit edge is the indicator instead: full height, accent, and it
	   is the same thing the pointer gets. */
	.resizer:focus-visible {
		outline: none;
	}
	.resizer:focus-visible::after {
		background: var(--accent);
	}

	@media (prefers-reduced-motion: reduce) {
		.resizer::after {
			transition: none;
		}
	}
</style>
