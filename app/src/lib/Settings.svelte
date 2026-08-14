<script lang="ts">
	import { open } from '@tauri-apps/plugin-dialog';
	import Icon from './Icon.svelte';
	import { archive, type LoreConfig, type Paths } from './archive';
	import { collector } from './collector.svelte';

	let { onClose }: { onClose: () => void } = $props();

	let config = $state<LoreConfig | null>(null);
	let error = $state<string | null>(null);
	let saved = $state(false);
	let where = $state<Paths | null>(null);

	$effect(() => {
		archive
			.config()
			.then((c) => (config = c))
			.catch((e: Error) => (error = e.message));
		archive
			.paths()
			.then((p) => (where = p))
			.catch(() => {});
	});

	async function save() {
		if (!config) return;
		try {
			config = await archive.saveConfig($state.snapshot(config));
			error = null;
			saved = true;
			setTimeout(() => (saved = false), 2000);
		} catch (e) {
			error = (e as Error).message;
		}
	}

	async function addRoot() {
		const picked = await open({ directory: true, multiple: false, title: 'Add a scanned root' });
		if (typeof picked !== 'string' || !config) return;
		const name = picked.split('/').filter(Boolean).pop() ?? 'uncategorized';
		config.roots = [...config.roots, { path: picked, category: name.toLowerCase() }];
		collector.needsScan = true;
		await save();
	}

	async function removeRoot(path: string) {
		if (!config) return;
		config.roots = config.roots.filter((r) => r.path !== path);
		collector.needsRebuild = true;
		await save();
	}

	let identityDraft = $state('');
	function addIdentity() {
		const value = identityDraft.trim().toLowerCase();
		if (!value || !config || config.identities.includes(value)) return;
		config.identities = [...config.identities, value];
		identityDraft = '';
		collector.needsScan = true;
		void save();
	}
</script>

<section class="settings">
	<div class="bar" data-tauri-drag-region>
		<button class="back" onclick={onClose}>
			<Icon name="back" size={18} />
			<span>Timeline</span>
		</button>
		<span class="crumb">Settings</span>
		<button class="close" onclick={onClose} aria-label="Close settings (Escape)">
			<Icon name="close" size={17} />
		</button>
	</div>

	<div class="scroll">
		{#if error}
			<p class="banner bad"><Icon name="warn" size={16} /><span>{error}</span></p>
		{/if}

		{#if config}
			<section class="group">
				<h2>Scanned roots</h2>
				<p class="note">
					Where lore looks for git repositories and file changes. Claude Code's own
					directory is a fixed location and is never configured. A category comes from the
					root a project sits under.
				</p>
				{#if config.roots.length === 0}
					<p class="empty">
						No roots. Only Claude Code is being read — git history and file changes are
						not.
					</p>
				{/if}
				<ul class="roots">
					{#each config.roots as root (root.path)}
						<li>
							<span class="swatch" data-category={root.category} aria-hidden="true"></span>
							<span class="path" title={root.path}>{root.path}</span>
							<input
								class="cat"
								bind:value={root.category}
								onchange={() => {
									collector.needsRebuild = true;
									void save();
								}}
								aria-label="Category"
							/>
							<button class="remove" onclick={() => removeRoot(root.path)} aria-label="Remove root">
								<Icon name="close" size={14} />
							</button>
						</li>
					{/each}
				</ul>
				<button class="act" onclick={addRoot}>Add a root…</button>
			</section>

			<section class="group">
				<h2>Timing</h2>
				<label class="field">
					<span class="label">Scan every</span>
					<input
						type="number"
						min="1"
						bind:value={config.scan_interval_mins}
						onchange={() => void save()}
					/>
					<span class="unit">minutes</span>
					<span class="why">
						Scanning happens while this window is open, and only then. Nothing runs in
						the background, so lore has to be opened at least once every 30 days or so —
						that is how long the sources keep their own history before deleting it.
					</span>
				</label>

				<label class="field">
					<span class="label">A pause ends a block after</span>
					<input
						type="number"
						min="1"
						bind:value={config.idle_gap_mins}
						onchange={() => {
							collector.needsRebuild = true;
							void save();
						}}
					/>
					<span class="unit">minutes</span>
					<span class="why">
						Longer means fewer, longer blocks and more counted time — a coffee break stops
						splitting the day. Changing it re-derives; nothing is re-read.
					</span>
				</label>

				<label class="field">
					<span class="label">First scan looks back</span>
					<input
						type="number"
						min="1"
						bind:value={config.file_lookback_days}
						onchange={() => void save()}
					/>
					<span class="unit">days</span>
					<span class="why">
						A modified time records only the last save, so older ones say little beyond
						"untouched since".
					</span>
				</label>
			</section>

			<section class="group">
				<h2>Your git identities</h2>
				<p class="note">
					Commits by these addresses are archived as yours. Repository and global git config
					are read automatically; add any address you use on another machine. Everyone
					else's commits are skipped — without this, a clone would fill the record with
					other people's work.
				</p>
				<ul class="identities">
					{#each config.identities as identity (identity)}
						<li>
							<span class="mono">{identity}</span>
							<button
								class="remove"
								aria-label="Remove identity"
								onclick={() => {
									config!.identities = config!.identities.filter((i) => i !== identity);
									collector.needsScan = true;
									void save();
								}}
							>
								<Icon name="close" size={14} />
							</button>
						</li>
					{/each}
				</ul>
				<div class="addrow">
					<input
						bind:value={identityDraft}
						placeholder="you@example.com"
						onkeydown={(e) => e.key === 'Enter' && addIdentity()}
					/>
					<button class="act" onclick={addIdentity}>Add</button>
				</div>
			</section>

			<section class="group">
				<h2>Excluded directories</h2>
				<p class="note">
					Pruned during the walk. Inside a repository lore asks git instead, so
					<span class="mono">.gitignore</span> already applies there.
				</p>
				<p class="chips">
					{#each config.exclude as name (name)}<span class="chip">{name}</span>{/each}
				</p>
			</section>

			<section class="group">
				<h2>Collector</h2>
				<p class="note">
					A scan reads the sources and archives what is new; it runs on its own while
					this window is open, and these force one now. A rebuild recomputes blocks,
					sessions and links from records already archived — it reads nothing and can
					lose nothing.
				</p>
				<div class="actions">
					<button
						class="act strong"
						disabled={!!collector.busy}
						onclick={() => collector.run('scan')}
					>
						{collector.busy === 'scan' ? 'Scanning…' : 'Scan now'}
					</button>
					<button
						class="act"
						disabled={!!collector.busy}
						onclick={() => collector.run('rebuild')}
					>
						{collector.busy === 'rebuild' ? 'Rebuilding…' : 'Rebuild'}
					</button>
					{#if collector.needsScan}
						<span class="pending">A root or identity changed — scan to read it.</span>
					{:else if collector.needsRebuild}
						<span class="pending">A grouping setting changed — rebuild to apply it.</span>
					{:else if saved}
						<span class="ok">Saved</span>
					{/if}
				</div>
				{#if collector.error}
					<p class="banner bad"><Icon name="warn" size={16} /><span>{collector.error}</span></p>
				{/if}
				{#if collector.result}
					<pre class="result">{collector.result}</pre>
				{/if}
			</section>

			{#if where}
				<section class="group">
					<h2>Where things live</h2>
					<dl class="paths">
						<dt>Config</dt>
						<dd class="mono">{where.config_path}</dd>
						<dt>Archive</dt>
						<dd class="mono">{where.db_path}</dd>
					</dl>
				</section>
			{/if}
		{:else if !error}
			<p class="note">Reading the configuration…</p>
		{/if}
	</div>
</section>

<style>
	.settings {
		display: flex;
		flex-direction: column;
		flex: 1;
		min-width: 0;
		min-height: 0;
		background: var(--ground);
	}

	.bar {
		display: flex;
		align-items: center;
		gap: 14px;
		flex: none;
		height: calc(var(--titlebar) + 18px);
		padding: 12px 18px 0;
		border-bottom: 1px solid var(--line);
	}
	.back {
		display: flex;
		align-items: center;
		gap: 5px;
		padding: 5px 10px 5px 6px;
		margin-left: 62px;
		border-radius: var(--radius-sm);
		color: var(--text-dim);
		font-size: 14px;
		font-weight: 540;
	}
	.back:hover {
		background: var(--surface-hover);
		color: var(--text);
	}
	.crumb {
		flex: 1;
		font-size: 14px;
		font-weight: 560;
		color: var(--text-dim);
	}
	.close {
		padding: 6px;
		border-radius: var(--radius-sm);
		color: var(--text-faint);
	}
	.close:hover {
		background: var(--surface-hover);
		color: var(--text);
	}

	.scroll {
		flex: 1;
		min-height: 0;
		overflow-y: auto;
		padding: 0 24px 64px;
	}

	.group {
		max-width: 74ch;
		margin: 0 auto;
		padding: 26px 0;
		border-bottom: 1px solid var(--line);
	}
	.group:last-child {
		border-bottom: none;
	}

	h2 {
		margin: 0 0 8px;
		font-size: 15.5px;
		font-weight: 620;
		letter-spacing: -0.01em;
	}

	.note {
		margin: 0 0 14px;
		max-width: 68ch;
		font-size: 13.5px;
		line-height: 1.55;
		color: var(--text-faint);
	}

	.empty {
		margin: 0 0 12px;
		font-size: 13.5px;
		color: var(--amber);
	}

	.banner {
		display: flex;
		align-items: flex-start;
		gap: 9px;
		max-width: 74ch;
		margin: 22px auto 0;
		padding: 11px 13px;
		border-radius: var(--radius-sm);
		font-size: 13.5px;
		line-height: 1.5;
	}
	.banner span {
		flex: 1;
	}
	.banner.bad {
		background: rgba(229, 85, 92, 0.14);
		color: var(--del);
	}

	.roots,
	.identities {
		margin: 0 0 12px;
		padding: 0;
		list-style: none;
		display: flex;
		flex-direction: column;
		gap: 6px;
	}
	.roots li,
	.identities li {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 8px 10px;
		border: 1px solid var(--line);
		border-radius: var(--radius-sm);
		background: var(--surface);
	}

	.swatch {
		width: 9px;
		height: 9px;
		flex: none;
		border-radius: 3px;
		background: var(--text-faint);
	}
	.swatch[data-category='work'] {
		background: var(--cat-work);
	}
	.swatch[data-category='research'] {
		background: var(--cat-research);
	}
	.swatch[data-category='personal'] {
		background: var(--cat-personal);
	}
	.swatch[data-category='freelance'] {
		background: var(--cat-freelance);
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

	input {
		border: 1px solid var(--line);
		border-radius: var(--radius-sm);
		background: var(--surface-inset);
		color: var(--text);
		font: inherit;
		font-size: 14px;
		padding: 5px 9px;
	}
	input:focus {
		outline: none;
		border-color: var(--accent-edge);
	}
	.cat {
		width: 122px;
		flex: none;
	}

	.remove {
		flex: none;
		display: grid;
		place-items: center;
		width: 24px;
		height: 24px;
		border-radius: var(--radius-sm);
		color: var(--text-faint);
	}
	.remove:hover {
		background: var(--surface-hover);
		color: var(--del);
	}

	.field {
		display: grid;
		grid-template-columns: 1fr auto auto;
		align-items: center;
		gap: 10px;
		padding: 10px 0;
		border-top: 1px solid var(--line);
	}
	.field .label {
		font-size: 14.5px;
		font-weight: 540;
	}
	.field input {
		width: 84px;
		text-align: right;
		font-family: var(--mono);
	}
	.unit {
		font-size: 13.5px;
		color: var(--text-faint);
	}
	.why {
		grid-column: 1 / -1;
		font-size: 13px;
		line-height: 1.5;
		color: var(--text-faint);
	}

	.addrow {
		display: flex;
		gap: 8px;
	}
	.addrow input {
		flex: 1;
	}


	.actions {
		display: flex;
		align-items: center;
		gap: 10px;
		flex-wrap: wrap;
	}
	.pending {
		font-size: 13px;
		color: var(--amber);
	}
	.ok {
		font-size: 13px;
		color: var(--add);
	}

	.result {
		margin: 12px 0 0;
		padding: 11px 13px;
		border-radius: var(--radius-sm);
		background: var(--surface-inset);
		font-family: var(--mono);
		font-size: var(--fs-min);
		line-height: 1.5;
		color: var(--text-dim);
		white-space: pre-wrap;
		overflow-x: auto;
	}

	.chips {
		display: flex;
		flex-wrap: wrap;
		gap: 6px;
		margin: 0;
	}
	.chip {
		padding: 3px 9px;
		border-radius: 999px;
		background: var(--surface-raised);
		font-family: var(--mono);
		font-size: var(--fs-min);
		color: var(--text-dim);
	}

	.mono {
		font-family: var(--mono);
		font-size: var(--fs-min);
	}

	.paths {
		margin: 0;
		display: grid;
		grid-template-columns: auto 1fr;
		gap: 6px 16px;
		font-size: 13.5px;
	}
	.paths dt {
		color: var(--text-faint);
	}
	.paths dd {
		margin: 0;
		color: var(--text-dim);
		overflow-wrap: anywhere;
	}
</style>
