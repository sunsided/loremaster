<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import type { ComponentProps } from 'svelte';
	import { fn } from 'storybook/test';
	import Button from './Button.svelte';

	const { Story } = defineMeta({
		title: 'Atoms/Button',
		component: Button,
		tags: ['autodocs'],
		args: { variant: 'secondary', size: 'default', onclick: fn() },
		argTypes: {
			variant: { control: 'inline-radio', options: ['primary', 'secondary', 'quiet'] },
			size: { control: 'inline-radio', options: ['default', 'compact'] },
			pressed: { control: 'boolean' },
			disabled: { control: 'boolean' }
		},
		parameters: {
			docs: {
				description: {
					component: `A button starts an action.

Use **primary** for the main action in a region. Use only one primary button in a region.
Use **secondary** for all other actions. Use **quiet** for low-priority actions, such as "Close" or "Cancel".

The **default** size is 44px high. Use it for main controls, such as verbs. The **compact** size is 32px high. Use it in rows of small actions.

Set \`pressed\` for a toggle button, such as a verb on the verb bar. The value goes to \`aria-pressed\`. The pressed look is the same as the primary look.

Set \`marked\` to show that the button is part of something else, for example a thing that the typed command names. A marked button has an accent edge and a faint accent fill. It is not pressed. Give screen readers the same fact in text, as ActionList does.

The button text must say what the button does. Do not use "Click here".`
				}
			}
		}
	});
</script>

{#snippet template({ children: _, ...args }: ComponentProps<typeof Button>)}
	<Button {...args}>Look at</Button>
{/snippet}

<Story name="Secondary" {template} />
<Story name="Primary" args={{ variant: 'primary' }} {template} />
<Story name="Quiet" args={{ variant: 'quiet' }} {template} />
<Story name="Compact" args={{ size: 'compact' }} {template} />
<Story name="Pressed" args={{ pressed: true }} {template} />
<Story name="Disabled" args={{ disabled: true }} {template} />
<Story name="Marked" args={{ marked: true, size: 'compact', variant: 'quiet' }} {template} />
