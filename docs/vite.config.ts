import { defineConfig } from 'vite'
import { ardo } from 'ardo/vite'
import { ferrocatReleaseVersion } from './release-version'

export default defineConfig({
  plugins: [
    ardo({
      title: 'Ferrocat',
      description: 'Performance-first translation catalogs for Gettext, ICU MessageFormat, and JSON-friendly runtime workflows.',
      project: {
        name: 'Ferrocat',
        version: ferrocatReleaseVersion,
      },

      // typedoc: true, // Uncomment to enable API docs

      // GitHub Pages: base path auto-detected from git remote

      sidebar: {
        sectionOrder: [
          'guide',
          'reference',
          'quality',
          'performance',
          'operations',
          'architecture',
          'notes',
          'archive',
        ],
      },
    }),
  ],
})
