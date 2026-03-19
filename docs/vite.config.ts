import { defineConfig } from 'vite'
import { ardo } from 'ardo/vite'

export default defineConfig({
  plugins: [
    ardo({
      title: 'Ferrocat',
      description: 'Built with Ardo',

      // typedoc: true, // Uncomment to enable API docs

      // GitHub Pages: base path auto-detected from git remote

      themeConfig: {
        siteTitle: 'Ferrocat',

        nav: [
          { text: 'Guide', link: '/guide/getting-started' },
          
        ],

        sidebar: [
          {
            text: 'Guide',
            items: [{ text: 'Getting Started', link: '/guide/getting-started' }],
          },
          
        ],

        footer: {
          message: 'Released under the MIT License.',
        },

        search: {
          enabled: true,
        },
      },
    }),
  ],
})
