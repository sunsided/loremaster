<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import VisuallyHidden from './VisuallyHidden.svelte';

	const { Story } = defineMeta({
		title: 'Atoms/VisuallyHidden',
		component: VisuallyHidden,
		tags: ['autodocs'],
		parameters: {
			docs: {
				description: {
					component: `VisuallyHidden puts text in the page for screen readers only. Sighted users do not see it.

Use it when the visual design gives information that has no text, for example a dash that means "hidden".
Do not use it to hide controls. A hidden control can get focus and confuse keyboard users.`
				}
			}
		}
	});
</script>

<Story name="In context" asChild>
	<p>
		<span aria-hidden="true">–</span>
		<VisuallyHidden>Fragment 3, hidden:</VisuallyHidden>
		Waves hiss on the stones.
	</p>
</Story>
