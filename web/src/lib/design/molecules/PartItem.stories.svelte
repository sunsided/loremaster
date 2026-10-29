<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import type { ComponentProps } from 'svelte';
	import PartItem from './PartItem.svelte';

	const { Story } = defineMeta({
		title: 'Molecules/PartItem',
		component: PartItem,
		tags: ['autodocs'],
		args: { label: 'oil lamp', text: 'An oil lamp sits on the table.', detail: 'portable', shown: true },
		parameters: {
			docs: {
				description: {
					component: `A part item shows one thing or one exit in a generated layer of the assembly panel. The runtime makes the objects layer from the things in the room, and the exits layer from the exits.

- A line in the room text now has a check mark.
- A line that is not in the room text has a dash, struck-through text and the reason. For a thing that is not listed, the runtime examines the room text. If the room text does not mention the thing, the reason says that the player cannot find it.
- \`detail\` gives more facts, for example the state of a door.

Screen readers hear "In the room text" or "Not in the room text" first. Put the item in a \`ul\`.`
				}
			}
		}
	});
</script>

{#snippet template(args: ComponentProps<typeof PartItem>)}
	<ul style="list-style: none; margin: 0; padding: 0; max-width: 420px">
		<PartItem {...args} />
	</ul>
{/snippet}

<Story name="Thing shown" {template} />
<Story
	name="Thing not listed"
	args={{ label: 'drawer', text: 'There is a drawer here.', detail: null, shown: false, why: 'not listed: the room text mentions it' }}
	{template}
/>
<Story
	name="Thing the player cannot find"
	args={{ label: 'brass key', text: 'There is a brass key here.', detail: 'portable', shown: false, why: 'not listed, and the room text does not mention it: the player cannot find it' }}
	{template}
/>
<Story name="Exit with a door" args={{ label: "north to Keeper's Cottage", text: 'north', detail: 'door: cottage door, closed' }} {template} />
<Story name="Hidden exit" args={{ label: 'through the mirror to Behind the Mirror', text: 'through the mirror', detail: null, shown: false, why: 'mirror_found is false' }} {template} />
