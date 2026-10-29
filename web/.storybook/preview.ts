import type { Preview } from '@storybook/sveltekit';
import { addons } from 'storybook/preview-api';
import '../src/lib/design/tokens/index.css';

function applyTheme(theme: unknown) {
	document.documentElement.dataset.theme = typeof theme === 'string' ? theme : 'phosphor';
}

// Docs pages have no story decorator, so follow the toolbar here too.
const channel = addons.getChannel();
channel.on('setGlobals', ({ globals }) => applyTheme(globals?.theme));
channel.on('globalsUpdated', ({ globals }) => applyTheme(globals?.theme));

const preview: Preview = {
	globalTypes: {
		theme: {
			description: 'Design theme',
			toolbar: {
				title: 'Theme',
				icon: 'paintbrush',
				items: [
					{ value: 'phosphor', title: 'Phosphor' },
					{ value: 'paper', title: 'Paper' },
					{ value: 'night', title: 'Night' }
				],
				dynamicTitle: true
			}
		}
	},
	initialGlobals: { theme: 'phosphor' },
	decorators: [
		(story, context) => {
			applyTheme(context.globals.theme);
			return story();
		}
	],
	parameters: {
		layout: 'padded',
		// Fail the a11y check on every violation, not only show it.
		a11y: { test: 'error' },
		backgrounds: { disable: true },
		controls: { matchers: { color: /(background|color)$/i } },
		options: {
			storySort: {
				order: ['Introduction', 'Foundations', 'Atoms', 'Molecules', 'Organisms', 'Templates']
			}
		}
	}
};

export default preview;
