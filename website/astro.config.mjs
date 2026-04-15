import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';

export default defineConfig({
  site: 'https://michellepellon.github.io',
  base: '/skillmark',
  integrations: [
    starlight({
      title: 'skillmark',
      description: 'CI-native linter, validator, and quality scorer for Agent Skills (SKILL.md)',
      social: [
        { icon: 'github', label: 'GitHub', href: 'https://github.com/michellepellon/skillmark' },
      ],
      editLink: {
        baseUrl: 'https://github.com/michellepellon/skillmark/edit/main/website/',
      },
      sidebar: [
        {
          label: 'Getting Started',
          items: [
            { slug: 'getting-started/installation' },
            { slug: 'getting-started/first-check' },
            { slug: 'getting-started/understanding-output' },
          ],
        },
        {
          label: 'Rules',
          items: [
            { slug: 'rules/errors' },
            { slug: 'rules/warnings' },
            { slug: 'rules/info' },
          ],
        },
        {
          label: 'Scoring',
          items: [
            { slug: 'scoring/how-scoring-works' },
            { slug: 'scoring/category-reference' },
          ],
        },
        {
          label: 'Guides',
          items: [
            { slug: 'guides/fix-mode' },
            { slug: 'guides/configuration' },
          ],
        },
        {
          label: 'Integration',
          items: [
            { slug: 'integration/github-action' },
            { slug: 'integration/pre-commit-hook' },
          ],
        },
        {
          label: 'Reference',
          items: [
            { slug: 'reference/cli' },
          ],
        },
      ],
      customCss: ['./src/styles/custom.css'],
      lastUpdated: true,
    }),
  ],
});
