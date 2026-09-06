#!/usr/bin/env node
// Guards the documentation host and the docs-site paths we advertise.
//
// The site is deployed to the ferrocat.dev apex domain with `basename: "/"`,
// so `https://sebastian-software.github.io/ferrocat/...` links can never
// resolve. This check fails on that host and verifies that every advertised
// `https://ferrocat.dev/<path>` link has a matching route under
// `docs/app/routes/`.

import { readFileSync, readdirSync, statSync } from "node:fs"
import { join, relative, resolve } from "node:path"

const repoRoot = resolve(import.meta.dirname, "..")
const routesRoot = resolve(repoRoot, "docs/app/routes")

const FORBIDDEN_HOST = "sebastian-software.github.io/ferrocat"
const SITE_LINK_PATTERN = /https:\/\/ferrocat\.dev(\/[^\s)"'`<>\]]*)?/g

const SCANNED_EXTENSIONS = new Set([".md", ".mdx", ".rs", ".ts", ".tsx", ".toml", ".yml", ".yaml"])
const SKIPPED_DIRECTORIES = new Set([".git", "build", "node_modules", "target"])

function collectFiles(directory) {
  const files = []
  for (const entry of readdirSync(directory, { withFileTypes: true })) {
    if (entry.isDirectory()) {
      if (SKIPPED_DIRECTORIES.has(entry.name)) continue
      files.push(...collectFiles(join(directory, entry.name)))
      continue
    }
    if (!entry.isFile()) continue
    const dot = entry.name.lastIndexOf(".")
    if (dot < 0 || !SCANNED_EXTENSIONS.has(entry.name.slice(dot))) continue
    files.push(join(directory, entry.name))
  }
  return files
}

function exists(path) {
  try {
    statSync(path)
    return true
  } catch {
    return false
  }
}

// Ardo derives routes from the file tree: `guide/getting-started.mdx` serves
// `/guide/getting-started`, and `performance/index.mdx` serves `/performance`.
function routeExists(routePath) {
  const segments = routePath.split("/").filter((segment) => segment !== "")
  if (segments.length === 0) return exists(join(routesRoot, "home.tsx"))
  if (segments.some((segment) => segment === "." || segment === "..")) return false

  const base = join(routesRoot, ...segments)
  return [".mdx", ".md", ".tsx"].some(
    (extension) => exists(base + extension) || exists(join(base, `index${extension}`)),
  )
}

const failures = []

for (const file of collectFiles(repoRoot)) {
  const displayPath = relative(repoRoot, file)
  const text = readFileSync(file, "utf8")

  if (text.includes(FORBIDDEN_HOST)) {
    failures.push(
      `${displayPath}: links to ${FORBIDDEN_HOST}; the docs site lives at https://ferrocat.dev/`,
    )
  }

  for (const [link, path = "/"] of text.matchAll(SITE_LINK_PATTERN)) {
    const routePath = path.split("#")[0].split("?")[0]
    if (!routeExists(routePath)) {
      failures.push(`${displayPath}: ${link} has no route under docs/app/routes/`)
    }
  }
}

if (failures.length > 0) {
  console.error("Documentation link check failed:")
  for (const failure of failures) console.error(`  - ${failure}`)
  process.exit(1)
}

console.log("Documentation link check passed.")
