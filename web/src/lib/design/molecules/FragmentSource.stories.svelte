<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import { shoreBasePieces } from '../fixtures.ts';
	import FragmentSource from './FragmentSource.svelte';

	const { Story } = defineMeta({
		title: 'Molecules/FragmentSource',
		component: FragmentSource,
		tags: ['autodocs'],
		args: { pieces: shoreBasePieces },
		parameters: {
			docs: {
				description: {
					component: `FragmentSource shows the source text of one fragment variant, with its inline markup. The runtime splits the source into pieces and marks each piece as active or not in the current state.

- Story text that applies now has the normal text colour.
- A branch that does not apply now is struck through. For example, \`{first: ...}\` is struck through after the first visit, and the day branch of \`{night: ... | ...}\` is struck through at night.
- Markup (braces, conditions and separators) is muted.
- An insert, such as \`{time.phase}\`, shows its current value in a small box after it.

Screen readers hear "not now:" before a struck-through piece, and "now" before the value of an insert. They do not announce the line or the box.`
				}
			}
		}
	});
</script>

<Story name="Second visit at dusk" />
<Story
	name="First visit at night"
	args={{
		pieces: [
			{ role: 'text', text: 'Shingle crunches underfoot. To the north a whitewashed cottage huddles against the wind', active: true },
			{ role: 'syntax', text: '{first: ', active: true },
			{ role: 'text', text: ', its windows dark', active: true },
			{ role: 'syntax', text: '}', active: true },
			{ role: 'text', text: '. East, the lighthouse rises ', active: true },
			{ role: 'syntax', text: '{night: ', active: true },
			{ role: 'text', text: 'black against the stars', active: true },
			{ role: 'syntax', text: ' | ', active: true },
			{ role: 'text', text: 'pale against the ', active: false },
			{ role: 'insert', text: '{time.phase}', active: false, value: 'night' },
			{ role: 'text', text: ' sky', active: false },
			{ role: 'syntax', text: '}', active: true },
			{ role: 'text', text: '.', active: true }
		]
	}}
/>
