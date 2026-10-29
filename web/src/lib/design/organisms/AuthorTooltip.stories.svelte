<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import type { Inspect } from '#lib/engine/types.ts';
	import ProseLink from '../atoms/ProseLink.svelte';
	import AuthorTooltip from './AuthorTooltip.svelte';

	const facts: Record<string, Inspect> = {
		'thing:oil_lamp': {
			kind: 'thing',
			id: 'oil_lamp',
			title: 'oil lamp',
			facts: [
				['id', 'oil_lamp'],
				['also called', 'lamp, lantern'],
				['location', "Keeper's Cottage (cottage)"],
				['kind', 'portable'],
				['lit', 'false']
			],
			rules: ['lamp_needs_matches', 'light_the_beacon']
		},
		'exit:north': {
			kind: 'exit',
			id: 'north',
			title: 'Exit north',
			facts: [
				['leads to', "Keeper's Cottage (cottage)"],
				['door', 'cottage_door, closed'],
				['way back', 'yes']
			],
			rules: []
		}
	};

	const { Story } = defineMeta({
		title: 'Organisms/AuthorTooltip',
		component: AuthorTooltip,
		tags: ['autodocs'],
		parameters: {
			docs: {
				description: {
					component: `An author tooltip shows facts about a thing or an exit: its id, names, location, properties, door state, conditions and related rules. It is for the author, not the player.

To see it, move the pointer onto a thing or an exit in the story text or on the verb bar, or move keyboard focus to it. A click still plays the game.

- The tooltip shows after a short delay on hover, and at once on focus.
- It stays while the pointer is on it, so you can read it (WCAG 1.4.13).
- Escape closes it.
- Screen readers hear its text as the description of the element (\`aria-describedby\`).
- The "Look and motion" settings and the command palette can turn it off.

An element gets a tooltip when it has \`data-inspect\`, for example \`data-inspect="thing:oil_lamp"\` or \`data-inspect="exit:north"\`. ProseLink and ActionList set it from their props.`
				}
			}
		}
	});
</script>

<Story name="Hover or focus a link" asChild>
	<p style="font-family: var(--font-prose); font-size: var(--font-size-prose); max-width: var(--measure-prose)">
		An <ProseLink inspect="thing:oil_lamp" onclick={() => {}}>oil lamp</ProseLink> sits on the table. Exits:
		<ProseLink kind="exit" inspect="exit:north" onclick={() => {}}>north</ProseLink>.
	</p>
	<AuthorTooltip inspect={(kind, id) => facts[`${kind}:${id}`] ?? null} />
</Story>
