<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import type { RustPart } from '#lib/engine/types.ts';
	import RustPartsView from './RustPartsView.svelte';

	const part = (line: number, overridden_by: RustPart['overridden_by'] = null): RustPart => ({
		origin: { kind: 'rust', file: 'src/world.rs', line, column: 5, pack: 'world', hash: 'af66c5' },
		value: { id: 'shore', name: 'The Shore' },
		commit: null,
		overridden_by
	});
	const yes = () => true;
	const done = async () => null;

	const { Story } = defineMeta({
		title: 'Organisms/RustPartsView',
		component: RustPartsView,
		tags: ['autodocs'],
		parameters: {
			docs: {
				description: {
					component: `The parts of a story that are made in Rust, in Atlas › Rust. Each part is read-only and shows its source line.

**Override** asks for a confirmation and writes a YAML file with the base of the part. **Eject** writes a YAML file with \`replace: true\`.`
				}
			}
		}
	});
</script>

<Story name="All states">
	<RustPartsView
		parts={{
			'room:shore': part(11),
			'room:cottage': part(16, ['rooms/cottage.yaml', 'active']),
			'thing:lamp': part(21, ['things/lamp.yaml', 'no_longer_needed']),
			'room:cellar': part(30)
		}}
		origins={{ 'room:shore': 'src/world.rs', 'room:cottage': 'rooms/cottage.yaml', 'thing:lamp': 'things/lamp.yaml', 'room:cellar': 'rooms/cellar.yaml' }}
		canOverride={yes}
		canEject={yes}
		oneject={done}
		onoverride={done}
		ondelete={done}
	/>
</Story>
