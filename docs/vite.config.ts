import { defineConfig } from 'vite'
import { ardo } from 'ardo/vite'

export default defineConfig({
  plugins: [
    ardo({
      title: 'Ferrocat',
      description: 'Performance-first translation catalogs for Gettext, ICU MessageFormat, and JSON-friendly runtime workflows.',

      // typedoc: true, // Uncomment to enable API docs

      // GitHub Pages: base path auto-detected from git remote

      themeConfig: {
        siteTitle: 'Ferrocat Docs',

        nav: [
          { text: 'Guide', link: '/guide' },
          { text: 'Reference', link: '/reference' },
          { text: 'Quality', link: '/quality' },
          { text: 'Performance', link: '/performance' },
          { text: 'Architecture', link: '/architecture' },
        ],

        sidebar: [
          {
            text: 'Guide',
            items: [
              { text: 'Overview', link: '/guide' },
              { text: 'Getting Started', link: '/guide/getting-started' },
              { text: 'Catalog Modes', link: '/guide/catalog-modes' },
              { text: 'Runtime Compilation', link: '/guide/runtime-compilation' },
              { text: 'Project Status', link: '/guide/project-status' },
              { text: 'Community', link: '/guide/community' },
            ],
          },
          {
            text: 'Reference',
            items: [
              { text: 'Overview', link: '/reference' },
              { text: 'API Overview', link: '/reference/api-overview' },
            ],
          },
          {
            text: 'Quality',
            items: [
              { text: 'Overview', link: '/quality' },
              { text: 'Conformance', link: '/quality/conformance' },
              { text: 'Test Coverage', link: '/quality/test-coverage' },
            ],
          },
          {
            text: 'Performance',
            items: [
              { text: 'Overview', link: '/performance' },
              { text: 'External Benchmarking', link: '/performance/benchmarking' },
              { text: 'Benchmark Fixtures', link: '/performance/benchmark-fixtures' },
              { text: 'Performance History', link: '/performance/performance-history' },
            ],
          },
          {
            text: 'Operations',
            items: [
              { text: 'Overview', link: '/operations' },
              { text: 'Release Verification', link: '/operations/release-verification' },
              { text: 'Migration Inventory', link: '/operations/migration-inventory' },
            ],
          },
          {
            text: 'Architecture',
            items: [
              { text: 'Overview', link: '/architecture' },
              { text: 'ADR Index', link: '/architecture/adr' },
            ],
          },
          {
            text: 'Notes',
            items: [
              { text: 'Overview', link: '/notes' },
              { text: 'Scan Architecture', link: '/notes/2026-03-14-scan-architecture' },
              { text: 'Bundler-Aware Message Sidecars', link: '/notes/2026-03-17-bundler-aware-message-sidecars' },
            ],
          },
          {
            text: 'Archive',
            items: [
              { text: 'Overview', link: '/archive' },
              { text: 'Plans', link: '/archive/plans' },
              { text: 'Porting Plan (2026-03-14)', link: '/archive/plans/2026-03-14-ferrocat-porting-plan' },
            ],
          },
        ],

        footer: {
          message: 'Ferrocat documentation for performance-first localization tooling.',
        },

        search: {
          enabled: true,
        },
      },
    }),
  ],
})
