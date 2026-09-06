import type { MetaFunction } from "react-router";

import {
  ArdoFooter,
  ArdoGeneratedSidebar,
  ArdoHeader,
  ArdoNav,
  ArdoNavLink,
  ArdoRoot,
  ArdoSidebar,
  ArdoSidebarSection,
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

import "./styles/site.css";

export { ArdoRootLayout as Layout } from "ardo/ui";

export const meta: MetaFunction = () => [{ title: config.title }];

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

export default function Root() {
  return (
    <ArdoRoot config={config}>
      <ArdoHeader title={config.title}>
        <ArdoNav>
          <ArdoNavLink to="/guide/getting-started">Guide</ArdoNavLink>
          <ArdoNavLink to="/reference/api-overview">API</ArdoNavLink>
          <ArdoNavLink to="/performance/benchmarking">Performance</ArdoNavLink>
          <ArdoNavLink to="/architecture/adr">ADRs</ArdoNavLink>
        </ArdoNav>
      </ArdoHeader>

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

      <ArdoFooter>
        <p className="ferro-footer-note">
          Ferrocat v{ferrocatReleaseVersion}
          <span>
            Performance-first localization tooling for Gettext, ICU MessageFormat, and JSON-oriented
            delivery.
          </span>
        </p>
        <p className="ferro-footer-family">
          Part of <a href="https://ferramenta.dev">Ferramenta</a>, the family of Rust-native
          developer tools by <a href="https://oss.sebastian-software.com">Sebastian Software</a>.
        </p>
      </ArdoFooter>
    </ArdoRoot>
  );
}
