/** Category colour, derived rather than declared.
 *
 *  Categories come from the scanned roots, which are configuration: a person can
 *  name one anything and add a twelfth whenever they like. The build used to
 *  match four literal names in thirty-two CSS selectors, so a fifth root, a
 *  rename, or a capital letter dropped a category to neutral grey with nothing
 *  said. Colour is looked up here instead.
 */

/** Ten hues, derived from this system's own colours rather than from a ramp.
 *
 *  The first attempt at a palette failed for a reason worth recording: it was
 *  generated at one flat lightness and chroma across the wheel, which is
 *  mechanically even and looks nothing like lore. This system's colours are not
 *  flat — its warm tones sit light (`uncertain-amber` at L 0.79, `git-add` at
 *  0.75) and its cool ones deep and saturated (`accent-magenta` at L 0.64 /
 *  C 0.22, the old category blue at 0.66 / 0.18). A palette ignoring that curve
 *  produces colours that are individually fine and collectively foreign.
 *
 *  So the curve is the source. Lightness and chroma are interpolated between
 *  lore's own chromatic tokens at the nearest hues, and only the hue is chosen
 *  freely. Every entry is a sibling of a colour already in the system.
 *
 *  Two hues are withheld, both measured — nothing sits within 22° of
 *  `accent-magenta` or `uncertain-amber`. Magenta is selection, and `research`
 *  having been its exact value is why a selected rose bar looked identical to an
 *  unselected one. Amber is withheld for a narrower reason: the saves treatment
 *  draws amber end caps *inside* a mark, so an amber mark would swallow them.
 *  `git-del`'s red is left out too — it sits beside the +/− figures in Stream
 *  and Detail — while `git-add`'s green stays, as it served as a category hue
 *  here for a long time without confusion.
 *
 *  `solid` carries the mark; `tint` is its lighter sibling, used where a solid
 *  hue is not legible small — the density ramp on a day cell, a chip's label.
 *  Every solid clears 3:1 against `ground` even at the density floor, and the
 *  nearest pair is 25° apart. */
const PALETTE: { solid: string; tint: string }[] = [
	/* amber-orange */ { solid: '#f37e45', tint: '#ffbf9f' },
	/* citron       */ { solid: '#c4ba3a', tint: '#e6e29e' },
	/* leaf         */ { solid: '#94c45e', tint: '#cbeaae' },
	/* jade         */ { solid: '#54c98b', tint: '#b0efc8' },
	/* teal         */ { solid: '#00c4b3', tint: '#8deade' },
	/* cyan         */ { solid: '#00b9d7', tint: '#7de1f2' },
	/* sky          */ { solid: '#00a6f2', tint: '#82d3ff' },
	/* cobalt       */ { solid: '#508cff', tint: '#99c0ff' },
	/* iris         */ { solid: '#9274fc', tint: '#bdb2ff' },
	/* violet       */ { solid: '#c05bde', tint: '#dea3f1' }
];

const NEUTRAL = { solid: 'var(--text-faint)', tint: 'var(--text-dim)' };

/** The seed the palette is dealt from.
 *
 *  Sorting alone gives one arrangement, and if you dislike the pairing it hands
 *  you there is nothing to be done about it short of renaming a root. The seed
 *  is the way out: it permutes the palette, so the same categories can be dealt
 *  a different set of colours on request. It holds a display preference and
 *  nothing else — the same place, and the same kind of thing, as which view and
 *  range the window reopens on. */
const SEED_KEY = 'lore.hueSeed';

function storedSeed(): number {
	if (typeof localStorage === 'undefined') return 0;
	const raw = Number(localStorage.getItem(SEED_KEY));
	return Number.isFinite(raw) ? raw >>> 0 : 0;
}

let seed = $state(storedSeed());

/** Deal the palette again. Random rather than incremental: a shuffle promises a
 *  rearrangement, not the next one along, and one press should not have to be
 *  pressed nine more times to escape a pairing. */
export function shuffleHues(): void {
	seed = Math.floor(Math.random() * 0xffffffff) >>> 0;
	if (typeof localStorage !== 'undefined') {
		try {
			localStorage.setItem(SEED_KEY, String(seed));
		} catch {
			// Unwritable storage costs the arrangement at the next launch, nothing more.
		}
	}
}

/** mulberry32 — small, fast, and good enough to shuffle ten items. */
function random(from: number): () => number {
	let a = from >>> 0;
	return () => {
		a = (a + 0x6d2b79f5) >>> 0;
		let t = Math.imul(a ^ (a >>> 15), 1 | a);
		t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
		return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
	};
}

/** The palette in the order this seed deals it. A permutation, so every colour
 *  stays available and no two categories collide that would not have collided
 *  before — a rotation would only ever slide the same pairing sideways. */
const dealt = $derived.by(() => {
	const out = PALETTE.slice();
	const next = random(seed);
	for (let i = out.length - 1; i > 0; i--) {
		const j = Math.floor(next() * (i + 1));
		[out[i], out[j]] = [out[j], out[i]];
	}
	return out;
});

/** Every category the configuration defines, sorted, assigned by position.
 *
 *  Two earlier attempts were worse. Hashing a name straight to a slot collides
 *  about seventy percent of the time with five categories in ten slots — the
 *  birthday problem, not bad luck — and the first real config hit it, giving
 *  `work` and `freelance` one hue. Assigning on first sight and remembering the
 *  result fixed the collision but made the colour depend on which category was
 *  drawn first, and parked the answer in `localStorage`, where a desktop app
 *  with a real config file has no business keeping it.
 *
 *  Sorting removes both faults at once. The scanned roots are the authoritative
 *  list and they do not change with the range on screen, so ordering them by
 *  name gives every category a colour that depends on nothing but the set of
 *  names — not on draw order, not on config order, not on anything stored. Add
 *  a root and the categories after it alphabetically shift by one; that is the
 *  only time any colour moves, and it is a moment when the palette is expected
 *  to be redealt.
 */
let order = $state<string[]>([]);

/** Called once the configuration is known.
 *
 *  `uncategorized` is dropped rather than sorted in or appended. The collector
 *  uses it for a path under no configured root, so it is not a category anyone
 *  named — it is the absence of one, and it takes no hue. That matters more than
 *  it sounds: it is routinely the largest bucket, so giving it a colour spent
 *  the loudest thing on screen on the one group carrying the least meaning, and
 *  cost a real category a slot in the bargain. White would be worse still,
 *  `text` being the brightest ink the system has. */
export function setCategories(names: string[]): void {
	const seen = new Set(
		names.map((n) => n.trim().toLowerCase()).filter((n) => n && n !== 'uncategorized')
	);
	order = [...seen].sort((a, b) => a.localeCompare(b));
}

/** The mark colour for a category, and its lighter sibling. Internal: every
 *  caller goes through `hueStyle`, so there is one way to spend a hue.
 *
 *  A category absent from the configuration takes no hue. That covers
 *  `uncategorized`, the moment before the config has loaded, and a category left
 *  in the archive by a root that has since been removed — none has earned a
 *  colour, and neutral says so rather than borrowing one. */
function hueFor(category: string | null | undefined): {
	solid: string;
	tint: string;
} {
	const key = category?.trim().toLowerCase();
	if (!key) return NEUTRAL;
	const at = order.indexOf(key);
	if (at < 0) return NEUTRAL;
	// Past ten categories the palette wraps and two share a hue. The interface
	// does not hide that: they are told apart by the name beside them, never by
	// the mark alone.
	return dealt[at % dealt.length];
}

/** The two custom properties every category-coloured element sets, ready for a
 *  `style` attribute. Kept in one place so a component never spells out which
 *  property carries the hue. */
export function hueStyle(category: string | null | undefined): string {
	const { solid, tint } = hueFor(category);
	return `--cat: ${solid}; --cat-tint: ${tint}`;
}
