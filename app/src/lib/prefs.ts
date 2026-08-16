/** Display preferences: which view, range and shape the window reopens on, how
 *  the palette was dealt, and how wide the panes are.
 *
 *  None of it is archive data — losing the lot costs nobody their history — so
 *  unwritable storage is swallowed rather than surfaced.
 */
const PREFIX = 'athar.';

export function readPref(name: string): string | null {
	if (typeof localStorage === 'undefined') return null;
	return localStorage.getItem(PREFIX + name);
}

export function writePref(name: string, value: string): void {
	if (typeof localStorage === 'undefined') return;
	try {
		localStorage.setItem(PREFIX + name, value);
	} catch {
		// Unwritable storage costs the preference at the next launch, nothing more.
	}
}
