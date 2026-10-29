import type { StorybookConfig } from '@storybook/sveltekit';

const config: StorybookConfig = {
	stories: ['../src/**/*.mdx', '../src/**/*.stories.svelte'],
	addons: ['@storybook/addon-svelte-csf', '@storybook/addon-a11y', '@storybook/addon-docs'],
	framework: { name: '@storybook/sveltekit', options: {} },
	docs: { defaultName: 'Docs' }
};

export default config;
