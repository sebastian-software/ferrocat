import type { MetaFunction } from "react-router";

import { Mark, MarkDefs, SiteFooter, SiteHeader } from "@ferramenta/family";
import {
  ArdoGeneratedSidebar,
  ArdoRoot,
  ArdoSearch,
  ArdoSidebar,
  ArdoSidebarSection,
  ArdoThemeToggle,
} from "ardo/ui";
import {
  BookOpen,
  Box,
  CodeXml,
  FileCode,
  FileText,
  Package,
  Settings,
  Wrench,
} from "lucide-react";
import config from "virtual:ardo/config";

import { ferrocatReleaseVersion } from "../release-version";
import "ardo/ui/styles.css";
import "@ferramenta/family/tokens.css";
import "@ferramenta/family/fonts.css";
import "@ferramenta/family/theme.css";

import "./styles/site.css";

// Last on purpose (the package README states this order): the shared chrome has
// to win the selector ties the site stylesheet would otherwise take.
import "@ferramenta/family/chrome.css";

export { ArdoRootLayout as Layout } from "ardo/ui";

export const meta: MetaFunction = () => [{ title: config.title }];

/*
 * The family chrome replaces Ardo's own header and footer, so Ardo must not
 * render them: `chrome: false` is read from every route match, and no route
 * below overrides it. The sidebar is not part of that switch — the docs rail
 * and its generated navigation stay exactly as they were.
 */
export const handle = { chrome: false };

/*
 * Sidebar rail sections, in the order declared as `sidebar.sectionOrder` in
 * vite.config.ts. Each one owns a top-level route segment and renders the
 * sidebar Ardo generates from the files under `app/routes/<segment>/`.
 *
 * The icons stay on lucide-react. The family design system (ferramenta ADR
 * 0002) does not use lucide in its own UI, but it has no mark for a docs
 * navigation section either, so this is a documented exception until the
 * shared family theme lands.
 */
const sections = [
  { id: "guide", label: "Guide", to: "/guide", icon: BookOpen },
  { id: "reference", label: "Reference", to: "/reference", icon: CodeXml },
  { id: "quality", label: "Quality", to: "/quality", icon: Box },
  { id: "performance", label: "Performance", to: "/performance", icon: Wrench },
  { id: "operations", label: "Operations", to: "/operations", icon: Settings },
  { id: "architecture", label: "Architecture", to: "/architecture", icon: FileCode },
  { id: "notes", label: "Notes", to: "/notes", icon: FileText },
  { id: "archive", label: "Archive", to: "/archive", icon: Package },
] as const;

/*
 * The family header carries one slot (`themeToggle`, at the end of the bar), so
 * what Ardo's own header used to provide rides in it: a named section menu (the
 * sidebar rail shows icons only, and below 1024px it is hidden altogether) and
 * full-text search. `ArdoSearch` reads its index from a virtual module and
 * falls back to the default labels, so it works outside `ArdoRoot`'s provider.
 * Plain links, not `Link`: a full navigation is what closes the flyout again.
 */
function DocsTools() {
  return (
    <>
      <details className="ferro-sections">
        <summary aria-label="Documentation sections">
          Docs <Mark name="chev" className="chev icon" size={16} />
        </summary>
        <div className="ferro-sections-flyout">
          {sections.map(({ id, label, to, icon: Icon }) => (
            <a href={to} key={id}>
              <Icon size={18} strokeWidth={1.8} aria-hidden="true" />
              {label}
            </a>
          ))}
        </div>
      </details>
      <div className="ferro-header-search">
        <ArdoSearch />
      </div>
      <ArdoThemeToggle />
    </>
  );
}

export default function Root() {
  return (
    <>
      <MarkDefs />
      <SiteHeader current="ferrocat" themeToggle={<DocsTools />} />

      {/*
       * `ferro-shell` is the hook site.css needs to turn Ardo's fixed-viewport
       * app shell into a document-scrolling page: the family footer sits below
       * the shell, so the page — not the article — has to be what scrolls.
       */}
      <div className="ferro-shell">
        <ArdoRoot config={config}>
          <ArdoSidebar>
            {sections.map(({ id, label, to, icon: Icon }) => (
              <ArdoSidebarSection
                key={id}
                id={id}
                label={label}
                to={to}
                icon={<Icon size={18} strokeWidth={1.8} />}
              >
                <ArdoGeneratedSidebar section={id} />
              </ArdoSidebarSection>
            ))}
          </ArdoSidebar>
        </ArdoRoot>
      </div>

      <SiteFooter
        current="ferrocat"
        legal={
          <>
            Ferrocat v{ferrocatReleaseVersion} — dual-licensed under MIT or Apache-2.0. This site is
            MIT-licensed.
          </>
        }
      />
    </>
  );
}
