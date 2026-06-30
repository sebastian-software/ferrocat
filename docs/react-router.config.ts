import type { Config } from "@react-router/dev/config"

export default {
  ssr: false,
  prerender: true,
  // Served from the custom apex domain ferrocat.dev at the root path, so the
  // basename must stay "/". detectGitHubBasename() would derive "/ferrocat/"
  // from the repo name, which only fits the github.io/<repo>/ project URL.
  basename: "/",
} satisfies Config
