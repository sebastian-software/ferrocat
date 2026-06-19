import {
  ArdoRoot,
  ArdoSidebar,
  ArdoSidebarGroup,
  ArdoSidebarLink,
} from "ardo/ui"
import type { SidebarItem } from "ardo"
import config from "virtual:ardo/config"
import sidebar from "virtual:ardo/sidebar"
import type { MetaFunction } from "react-router"
import { version } from "../package.json"
import {
  BookOpen,
  Box,
  CodeXml,
  FileCode,
  FileText,
  Settings,
  Wrench,
} from "lucide-react"
import "ardo/ui/styles.css"
import "./styles/site.css"

export { ArdoRootLayout as Layout } from "ardo/ui"

export const meta: MetaFunction = () => [{ title: config.title }]

const sectionIcons = {
  Architecture: FileCode,
  Archive: BookOpen,
  Guide: BookOpen,
  Notes: FileText,
  Operations: Settings,
  Performance: Wrench,
  Quality: Box,
  Reference: CodeXml,
} satisfies Record<string, typeof BookOpen>

function getSectionIcon(section: string) {
  const Icon = sectionIcons[section as keyof typeof sectionIcons]
  return Icon == null ? undefined : <Icon size={18} strokeWidth={1.8} />
}

function renderSidebarItem(item: SidebarItem) {
  const key = item.link ?? item.text
  const childItems = item.items ?? []

  if (childItems.length > 0) {
    return (
      <ArdoSidebarGroup
        key={key}
        title={item.text}
        to={item.link}
        collapsed={item.collapsed}
        icon={getSectionIcon(item.text)}
      >
        {childItems.map(renderSidebarItem)}
      </ArdoSidebarGroup>
    )
  }

  if (item.link == null) {
    return (
      <ArdoSidebarGroup
        key={key}
        title={item.text}
        collapsed={item.collapsed}
        icon={getSectionIcon(item.text)}
      />
    )
  }

  return (
    <ArdoSidebarLink key={key} to={item.link}>
      {item.text}
    </ArdoSidebarLink>
  )
}

export default function Root() {
  return (
    <ArdoRoot
      config={config}
      sidebar={sidebar}
      sidebarContent={<ArdoSidebar>{sidebar.map(renderSidebarItem)}</ArdoSidebar>}
      footerProps={{
        ardoLink: false,
        project: undefined,
        children: (
          <p className="ferro-footer-note">
            Ferrocat Docs v{version}
            <span>Performance-first localization tooling for Gettext, ICU MessageFormat, and JSON-oriented delivery.</span>
          </p>
        ),
      }}
    />
  )
}
