<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import { fn } from 'storybook/test';
	import ProseLink from './ProseLink.svelte';

	const { Story } = defineMeta({
		title: 'Atoms/ProseLink',
		component: ProseLink,
		tags: ['autodocs'],
		args: { kind: 'thing', onclick: fn() },
		argTypes: { kind: { control: 'inline-radio', options: ['thing', 'exit'] } },
		parameters: {
			docs: {
				description: {
					component: `A prose link is a word in the story text that the player can click.

A **thing** link is an object, such as "oil lamp". A **thing** link has stronger text than the prose around it.
An **exit** link is a direction, such as "north". An **exit** link has the same colour as the prose.

The link is a \`button\`, not an \`a\` element, because it starts an action and does not go to a page. The keyboard can reach it with Tab. The underline shows that it is a control. Do not remove the underline.`
				}
			}
		}
	});
</script>

<Story name="Thing" asChild>
	<p style="font-family: var(--font-prose); font-size: var(--font-size-prose)">
		An <ProseLink onclick={() => {}}>oil lamp</ProseLink> sits on the table.
	</p>
</Story>
<Story name="Exit" asChild>
	<p style="font-family: var(--font-prose); font-size: var(--font-size-prose)">
		Exits: <ProseLink kind="exit" onclick={() => {}}>north</ProseLink>.
	</p>
</Story>
