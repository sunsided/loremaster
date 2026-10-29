<script lang="ts">
	import type { FragmentView, Layer } from '#lib/engine/index.ts';
	import type { Session } from './session.svelte.ts';

	let { session }: { session: Session } = $props();

	const layers: Layer[] = ['base', 'ambient', 'event', 'objects', 'characters', 'exits'];
	const generated: Partial<Record<Layer, string>> = {
		objects: 'listed from the things in the room',
		exits: 'listed from the visible exits'
	};

	const groups = $derived(
		layers.map((layer) => ({
			layer,
			items: session.assembly.filter((f) => f.layer === layer)
		}))
	);

	function meta(f: FragmentView): string {
		const parts: string[] = [f.mode.replace('_', ' ')];
		if (f.condition) parts.push(`when ${f.condition}`);
		if (f.replaces) parts.push('replaces');
		return parts.join(' · ');
	}
</script>

<aside aria-label="Assembly">
	<header>
		<span>ASSEMBLY · room.{session.scene.room}</span>
		<span>reads top to bottom</span>
	</header>
	<p class="intro">
		Shown fragments in the order the player reads them: layer by layer, then by position. Numbers
		match the sentences in the transcript.
	</p>

	{#each groups as g (g.layer)}
		<section>
			<div class="layer">
				<span class="name">{g.layer}</span>
				<span class="rule"></span>
			</div>
			<ul>
			{#each g.items as f (f.index)}
				<li
					class="frag"
					class:shown={f.shown}
					class:focus={session.focus === f.index}
					onmouseenter={() => (session.focus = f.index)}
					onmouseleave={() => (session.focus = null)}
				>
					<span class="n">{f.shown ? f.index + 1 : '–'}</span>
					<div>
						<div class="text">{f.text}</div>
						<div class="meta">{meta(f)}{#if f.why}{' · '}<span class="why">{f.why}</span>{/if}</div>
					</div>
				</li>
			{:else}
				<li class="empty">{generated[g.layer] ?? 'nothing in this layer'}</li>
			{/each}
			</ul>
		</section>
	{/each}

	<footer>
		<span>time {session.scene.time} · {session.scene.phase}</span>
		<span>editing in place comes next</span>
	</footer>
</aside>

<style>
	aside {
		flex: 1 1 380px;
		max-width: 480px;
		min-height: 0;
		box-sizing: border-box;
		border-left: 1px solid var(--line);
		background: var(--panel);
		padding: 18px 20px;
		display: flex;
		flex-direction: column;
		gap: 14px;
		overflow-y: auto;
	}
	@media (max-width: 900px) {
		aside {
			max-width: none;
			border-left: 0;
			border-top: 1px solid var(--line);
		}
	}
	header,
	footer {
		display: flex;
		justify-content: space-between;
		gap: 12px;
		color: var(--muted);
	}
	footer {
		margin-top: auto;
		border-top: 1px solid var(--line);
		padding-top: 12px;
		font-size: 12px;
	}
	.intro {
		margin: 0;
		color: var(--muted);
		line-height: 1.5;
	}
	section {
		display: flex;
		flex-direction: column;
		gap: 8px;
	}
	.layer {
		display: flex;
		align-items: center;
		gap: 8px;
	}
	.name {
		font-weight: 600;
		letter-spacing: 0.06em;
		text-transform: uppercase;
	}
	.rule {
		flex: 1 1 auto;
		height: 1px;
		background: var(--line);
	}
	ul {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 4px;
	}
	.frag {
		display: grid;
		grid-template-columns: 22px minmax(0, 1fr);
		gap: 8px;
		padding: 6px 8px;
		border: 1px solid transparent;
		color: var(--muted);
	}
	.frag.shown {
		color: var(--text);
	}
	.frag.focus {
		border-color: var(--warn);
	}
	.n {
		color: var(--warn-text);
	}
	.text {
		font-family: var(--prose);
		font-size: 14px;
		line-height: 1.45;
	}
	.frag:not(.shown) .text {
		text-decoration: line-through;
		text-decoration-color: var(--chip-line);
	}
	.meta {
		margin-top: 3px;
		font-size: 12px;
		color: var(--muted);
	}
	.why {
		color: var(--warn-text);
	}
	.empty {
		color: var(--muted);
		font-style: italic;
	}
</style>
