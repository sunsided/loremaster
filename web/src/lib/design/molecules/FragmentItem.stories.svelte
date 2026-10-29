<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import type { ComponentProps } from 'svelte';
	import { fn } from 'storybook/test';
	import FragmentItem from './FragmentItem.svelte';

	const { Story } = defineMeta({
		title: 'Molecules/FragmentItem',
		component: FragmentItem,
		tags: ['autodocs'],
		args: {
			number: 2,
			text: 'The sun is a red coal sinking into the sea.',
			meta: 'always · when time.phase == dusk',
			shown: true,
			active: false,
			onactivate: fn()
		},
		parameters: {
			docs: {
				description: {
					component: `A fragment item shows one fragment in the assembly panel: its number, its source text and its facts.

A **shown** fragment has its number. A **hidden** fragment has a dash, struck-through text and the reason why it is hidden.
The state is also in text for screen readers ("Fragment 3, hidden"). The design does not use colour as the only signal.

The item is a list item. Put it in a \`ul\`.`
				}
			}
		}
	});
</script>

{#snippet template(args: ComponentProps<typeof FragmentItem>)}
	<ul style="list-style: none; margin: 0; padding: 0; max-width: 420px">
		<FragmentItem {...args} />
	</ul>
{/snippet}

<Story name="Shown" {template} />
<Story name="Hidden" args={{ number: 3, shown: false, meta: 'shuffle · when time.phase == night', text: 'Waves hiss on the stones.', why: 'time.phase == night is false' }} {template} />
<Story name="Active" args={{ active: true }} {template} />
