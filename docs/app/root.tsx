import type { ReactNode } from "ardo/react";

import { ArdoActiveLink as NavLink } from "ardo/runtime";
import {
  MonoArdoErrorBoundary,
  MonoArdoLayout,
  MonoArdoSite,
  type MonoDocsSection,
} from "ferramenta-family/ardo";
import touchIcon from "ferramenta-family/ferra-assets/ferrocat/apple-touch-icon.png?url";
import favicon from "ferramenta-family/ferra-assets/ferrocat/favicon.ico?url";

import navigation from "./data/navigation.json";
import { version } from "./version";
import "ferramenta-family/monografie.css";
import "ferramenta-family/monografie-docs.css";
import "ferramenta-family/monografie-ardo.css";

export function Layout({ children }: { children: ReactNode }) {
  return (
    <MonoArdoLayout icon={favicon} appleTouchIcon={touchIcon}>
      {children}
    </MonoArdoLayout>
  );
}
export const ErrorBoundary = MonoArdoErrorBoundary;

const sections: MonoDocsSection[] = navigation.map((section) => ({
  id: section.id,
  label: section.label,
  to: section.to,
  pages: section.pages.map(({ title, to }) => ({ label: title, to })),
}));

const links = [
  <NavLink key="guide" to="/guide/getting-started">
    Guide
  </NavLink>,
  <NavLink key="reference" to="/reference/api-overview">
    Reference
  </NavLink>,
  <NavLink key="performance" to="/performance/results">
    Performance
  </NavLink>,
  <a key="api" href="https://docs.rs/ferrocat">
    API
  </a>,
];

/** Every page on Monografie's Ardo shell; the documentation adds its menu, search and reading layout. */
export default function Root() {
  return (
    <MonoArdoSite
      product="ferrocat"
      name="Ferrocat"
      release={version}
      links={links}
      repository="https://github.com/sebastian-software/ferrocat"
      sections={sections}
      homeClassName="fc-home"
      docsClassName="fc-docs"
      footer={
        <>
          <a href="https://github.com/sebastian-software/ferrocat">GitHub</a>
          <a href="https://crates.io/crates/ferrocat">crates.io</a>
          <a href="https://docs.rs/ferrocat">docs.rs</a>
        </>
      }
    />
  );
}
