<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import RadioGroup from './RadioGroup.svelte';

	const themes = [
		{ value: 'phosphor', label: 'Phosphor' },
		{ value: 'night', label: 'Night' },
		{ value: 'paper', label: 'Paper' }
	];
	const effects = [
		{ value: 'full', label: 'Full', description: 'Moving grain, scanlines, glow, idle flicker, glitch on new text, line reveal.' },
		{ value: 'subtle', label: 'Subtle', description: 'Still grain, light scanlines, soft glow, a short colour fringe on new text.' },
		{ value: 'off', label: 'Off', description: 'Flat colours and plain text. This is the fastest.' }
	];

	const { Story } = defineMeta({
		title: 'Molecules/RadioGroup',
		component: RadioGroup,
		tags: ['autodocs'],
		args: { legend: 'Theme', name: 'theme-demo', options: themes, value: 'phosphor', look: 'pills' },
		argTypes: { look: { control: 'inline-radio', options: ['pills', 'cards', 'list'] } },
		parameters: {
			docs: {
				description: {
					component: `A radio group lets the user select one choice from a small set.

The group is a \`fieldset\`. The \`legend\` is the question that the group answers. Screen readers announce the legend with each choice.

Use **pills** for two to four short choices. Use **cards** when each choice needs a description. Use **list** for long labels.
The arrow keys move between choices. This is the native behaviour of radio inputs.`
				}
			}
		}
	});
</script>

<Story name="Pills" />
<Story name="Cards" args={{ legend: 'Visual effects', name: 'fx-demo', options: effects, value: 'full', look: 'cards' }} />
<Story
	name="List"
	args={{
		legend: 'Motion',
		name: 'motion-demo',
		look: 'list',
		value: 'system',
		options: [
			{ value: 'system', label: 'Use my system setting (now: motion is permitted)' },
			{ value: 'reduce', label: 'Always reduce motion' }
		]
	}}
/>
