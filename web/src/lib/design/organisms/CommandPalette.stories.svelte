<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import Button from '../atoms/Button.svelte';
	import type { PaletteItem } from '../types.ts';
	import CommandPalette from './CommandPalette.svelte';

	const { Story } = defineMeta({
		title: 'Organisms/CommandPalette',
		component: CommandPalette,
		tags: ['autodocs'],
		parameters: {
			docs: {
				description: {
					component: `The command palette gives fast keyboard access to all actions. In the app, push **Ctrl+K** (or **Cmd+K** on a Mac) or click "Commands" in the header.

To use it:
1. Push Ctrl+K.
2. Type some words, for example "lantern" or "rule find".
3. Use the Up and Down arrow keys to select an action.
4. Push Enter to run it. Push Escape to close the palette.

The app gives these groups:
- **Inspector**: show a tab.
- **Rooms**: move the player to a room immediately. Leave and enter rules do not fire.
- **Rules**: show a rule card in the Rules tab and move focus to it.
- **States**: set a flag to its other value, or set an enum variable to a value.
- **Time**: set the time to the start of a phase.
- **Look** and **Game**: theme, effects, tooltips, settings and restart.
- **Play**: the last item runs the search text as a typed command.

Each word of the search must be in the action text. The best match is first. Actions that change the game state show in the transcript as "(author)" lines and in the Story tab.

The palette is a modal \`dialog\` with a combobox and a listbox (WAI-ARIA combobox pattern). Focus stays in the search field. The active option is \`aria-activedescendant\`.`
				},
				story: { inline: false, iframeHeight: 640 }
			}
		}
	});
</script>

<script lang="ts">
	let open = $state(false);
	let last = $state('nothing yet');
	const say = (text: string) => () => (last = text);
	const items: PaletteItem[] = [
		{ id: 'tab-rules', group: 'Inspector', label: 'Show Rules', run: say('Show Rules') },
		{ id: 'room-shore', group: 'Rooms', label: 'Go to The Shore', hint: 'you are here', run: say('Go to The Shore') },
		{ id: 'room-cottage', group: 'Rooms', label: "Go to Keeper's Cottage", hint: 'visited', run: say("Go to Keeper's Cottage") },
		{ id: 'rule-find', group: 'Rules', label: 'Show rule find_iron_key', hint: 'fired', run: say('Show rule find_iron_key') },
		{ id: 'var-beacon', group: 'States', label: 'Set beacon_lit to true', hint: 'now false', run: say('Set beacon_lit to true') },
		{ id: 'time-night', group: 'Time', label: 'Set the time to night (21:00)', run: say('Set the time to night') }
	];
	const fallback = (q: string): PaletteItem => ({ id: 'play', group: 'Play', label: `Do “${q}”`, hint: 'as a typed command', run: say(`Do ${q}`) });
</script>

<Story name="Open from a button" asChild>
	<Button aria-haspopup="dialog" onclick={() => (open = true)}>Commands</Button>
	<p>Last action: {last}</p>
	<CommandPalette bind:open {items} {fallback} />
</Story>
