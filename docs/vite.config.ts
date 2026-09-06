import { ardo } from "ardo/vite";
import { defineConfig } from "vite";

import { ferrocatReleaseVersion } from "./release-version";

export default defineConfig({
  // Served from the custom apex domain ferrocat.dev at the root path, so every
  // asset URL has to stay at "/".
  base: "/",
  plugins: [
    ardo({
      title: "Ferrocat",
      description:
        "Performance-first translation catalogs for Gettext, ICU MessageFormat, and JSON-friendly runtime workflows.",
      siteUrl: "https://ferrocat.dev",
      project: {
        name: "Ferrocat",
        version: ferrocatReleaseVersion,
      },

      // typedoc: true, // Uncomment to enable API docs

      // Turn off ardo's GitHub Pages auto-detection: it derives the base path
      // from the git remote and would prefix every asset URL with "/ferrocat/"
      // (the repo name), which only fits the github.io/<repo>/ project URL.
      // See also `basename` in react-router.config.ts.
      githubPages: false,

      sidebar: {
        sectionOrder: [
          "guide",
          "reference",
          "quality",
          "performance",
          "operations",
          "architecture",
          "notes",
          "archive",
        ],
      },
    }),
  ],
});
