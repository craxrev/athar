/** Display preferences, and the one place that knows they used to be `lore.*`.
 *
 *  These hold which view, range and shape the window reopens on, how the palette
 *  was dealt, and how wide the panes are. None of it is archive data — losing it
 *  costs nobody their history — but silently resetting every one of them on the
 *  launch after a rename is a bad way to be introduced to a new name.
 *
 *  The fallback is read once and rewritten under the new key, so the old one is
 *  consulted exactly until the first time a preference is saved.
 */
const LEGACY = 'lore.';
const PREFIX = 'athar.';

export function readPref(name: string): string | null {
	if (typeof localStorage === 'undefined') return null;
	const current = localStorage.getItem(PREFIX + name);
	if (current !== null) return current;
	const legacy = localStorage.getItem(LEGACY + name);
	if (legacy === null) return null;
	writePref(name, legacy);
	try {
		localStorage.removeItem(LEGACY + name);
	} catch {
		// Leaving the old key behind costs a few bytes and nothing else.
	}
	return legacy;
}

export function writePref(name: string, value: string): void {
	if (typeof localStorage === 'undefined') return;
	try {
		localStorage.setItem(PREFIX + name, value);
	} catch {
		// Unwritable storage costs the preference at the next launch, nothing more.
	}
}
