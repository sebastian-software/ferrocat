import { defineConfig } from 'vite'
import { ardo } from 'ardo/vite'

export default defineConfig({
  plugins: [
    ardo({
      title: 'Ferrocat',
      description: 'Performance-first translation catalogs for Gettext, ICU MessageFormat, and JSON-friendly runtime workflows.',

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
