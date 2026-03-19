import { RootLayout, ArdoRoot } from "ardo/ui"
import config from "virtual:ardo/config"
import sidebar from "virtual:ardo/sidebar"
import type { MetaFunction } from "react-router"
import "ardo/ui/styles.css"
import "./styles/site.css"

export const meta: MetaFunction = () => [{ title: config.title }]

export function Layout({ children }: { children: React.ReactNode }) {
  return <RootLayout>{children}</RootLayout>
}

export default function Root() {
  return (
    <ArdoRoot
      config={config}
      sidebar={sidebar}
      footerProps={{
        ardoLink: false,
        project: undefined,
        children: (
          <p className="ferro-footer-note">
            Ferrocat Docs v0.9.0
            <span>Performance-first localization tooling for Gettext, ICU MessageFormat, and JSON-oriented delivery.</span>
          </p>
        ),
      }}
    />
  )
}
