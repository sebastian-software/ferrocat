import { defineConfig } from 'vite'
import { ardo } from 'ardo/vite'
import { ferrocatReleaseVersion } from './release-version'

export default defineConfig({
  // Served from the custom apex domain ferrocat.dev at the root path. Setting
  // base explicitly disables ardo's GitHub Pages auto-detection, which would
  // otherwise prefix every asset URL with "/ferrocat/" (the repo name) and
  // break the site when it is not served from github.io/ferrocat/.
  base: '/',
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
