import { defineConfig } from 'vitepress'

const repository = 'https://github.com/gulfstream-rs/gulfstream'

export default defineConfig({
  title: 'Gulfstream',
  description: 'Configurable video ingestion, adaptive streaming, accounts, processing, and analytics.',
  lang: 'en-US',
  base: '/gulfstream/',
  cleanUrls: true,
  lastUpdated: true,
  sitemap: { hostname: 'https://gulfstream-rs.github.io/gulfstream/' },
  head: [
    ['meta', { name: 'theme-color', content: '#111827' }],
    ['meta', { name: 'repository', content: repository }],
  ],
  themeConfig: {
    logo: '/logo.svg',
    siteTitle: 'Gulfstream',
    nav: [
      { text: 'Guide', link: '/guide/quick-start' },
      { text: 'API', link: '/reference/api' },
      { text: 'Configuration', link: '/reference/configuration' },
      { text: 'Rustdoc', link: 'https://gulfstream-rs.github.io/gulfstream/rustdoc/gulfstream/' },
      { text: 'Repository', link: repository },
    ],
    sidebar: [
      {
        text: 'Guide',
        items: [
          { text: 'Quick start', link: '/guide/quick-start' },
          { text: 'Architecture', link: '/guide/architecture' },
          { text: 'Web interface', link: '/guide/web-interface' },
          { text: 'Accounts and authentication', link: '/guide/authentication' },
          { text: 'Uploads and imports', link: '/guide/uploads' },
          { text: 'Processing workflow', link: '/guide/processing' },
          { text: 'Playback and privacy', link: '/guide/playback' },
          { text: 'Analytics', link: '/guide/analytics' },
          { text: 'Operations', link: '/guide/operations' },
          { text: 'Live validation', link: '/guide/live-validation' },
          { text: 'Releasing', link: '/guide/releasing' },
          { text: 'Security', link: '/guide/security' },
        ],
      },
      {
        text: 'Reference',
        items: [
          { text: 'API endpoints', link: '/reference/api' },
          { text: 'Response examples', link: '/reference/responses' },
          { text: 'Configuration', link: '/reference/configuration' },
          { text: 'OpenAPI', link: '/reference/openapi' },
          { text: 'Environment variables', link: '/reference/environment' },
        ],
      },
      { text: 'Contributing', link: '/contributing' },
    ],
    socialLinks: [{ icon: 'github', link: repository }],
    editLink: {
      pattern: `${repository}/edit/main/docs/:path`,
      text: 'Edit this page on GitHub',
    },
    search: { provider: 'local' },
    footer: {
      message: 'Released under the MIT License.',
      copyright: 'Gulfstream contributors',
    },
  },
})
