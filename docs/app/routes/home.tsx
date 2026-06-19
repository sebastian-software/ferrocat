import {
  ArrowRight,
  BookOpenText,
  Boxes,
  ExternalLink,
  FileCheck2,
  FileText,
  Flame,
  GitBranch,
  GitMerge,
  Hammer,
  Languages,
  Package,
  Palette,
  Regex,
  ScanSearch,
  ShieldCheck,
  Spline,
  SpellCheck,
} from "lucide-react"
import type { MetaFunction } from "react-router"
import { Link } from "react-router"

export const meta: MetaFunction = () => [
  { title: "Ferrocat: Rust-native translation catalog engine" },
  {
    name: "description",
    content:
      "Parse, update, audit, and compile your translations with a Rust-native catalog engine. Missing strings, ICU mistakes, and stale copy fail CI instead of reaching production.",
  },
]

const GITHUB = "https://github.com/sebastian-software/ferrocat"
const COMPANY = "https://sebastian-software.com/"
const OSS = "https://oss.sebastian-software.com/"

// ── The Ferro family: every tool forged in Rust ──

type FamilyTool = {
  name: string
  role: string
  body: string
  icon: React.ReactNode
  href: string
  current?: boolean
}

const family: FamilyTool[] = [
  {
    name: "ferrocat",
    role: "Translation catalogs",
    body: "PO, NDJSON, and ICU MessageFormat with merge, audit, and runtime compilation.",
    icon: <Languages size={19} />,
    href: GITHUB,
    current: true,
  },
  {
    name: "ferromark",
    role: "Markdown to HTML",
    body: "CommonMark and every GFM extension at 309 MiB/s. Faster than pulldown-cmark and md4c.",
    icon: <FileText size={19} />,
    href: "https://github.com/sebastian-software/ferromark",
  },
  {
    name: "ferroni",
    role: "Regex engine",
    body: "Pure-Rust, Oniguruma-compatible. Same feature class, faster hot path, no C toolchain.",
    icon: <Regex size={19} />,
    href: "https://github.com/sebastian-software/ferroni",
  },
  {
    name: "ferrovia",
    role: "SVG optimizer",
    body: "SVGO-compatible output, verified differentially against svgo 4 on every build.",
    icon: <Spline size={19} />,
    href: "https://github.com/sebastian-software/ferrovia",
  },
  {
    name: "ferriki",
    role: "Syntax highlighting",
    body: "Shiki-compatible themes and grammars on a leaner Rust core, with Node bindings.",
    icon: <Palette size={19} />,
    href: "https://github.com/sebastian-software/ferriki",
  },
  {
    name: "ferrolex",
    role: "Spelling and dictionaries",
    body: "Spell, dictionary, and brand-term validation for code and localization workflows.",
    icon: <SpellCheck size={19} />,
    href: "https://github.com/sebastian-software/ferrolex",
  },
]

// ── What the catalog engine does for you ──

const benefits = [
  {
    title: "One catalog core, not ten formats",
    body: "Keep source text, context, plurals, comments, references, flags, and obsolete entries in a single model your application code can reason about.",
    icon: <Boxes size={20} />,
  },
  {
    title: "Updates without guesswork",
    body: "Merge freshly extracted messages into existing catalogs by exact identity. No fuzzy matching, no hidden ID changes, no silent conflict resolution.",
    icon: <GitMerge size={20} />,
  },
  {
    title: "Release QA that blocks bad ships",
    body: "Audit for missing locales, empty translations, stale targets, ICU errors, and metadata conflicts. The report says what is shippable, in diagnostics CI can read.",
    icon: <ShieldCheck size={20} />,
  },
  {
    title: "Rich messages that keep their values",
    body: "Analyze placeholders, formatters, plurals, selects, and tags so a translation can never quietly drop a runtime value the source needs.",
    icon: <ScanSearch size={20} />,
  },
  {
    title: "Runtime artifacts, not reparsing",
    body: "Compile locale-resolved payloads with stable keys and explicit fallback. Your app loads compiled data instead of parsing translator files in production.",
    icon: <Package size={20} />,
  },
  {
    title: "AI translation you can trust",
    body: "Track model, confidence, and a change hash for machine output. The metadata clears itself the moment a human edits the text.",
    icon: <FileCheck2 size={20} />,
  },
]

// ── The path a catalog takes through Ferrocat ──

const steps = [
  {
    label: "Parse",
    body: "Read PO or NDJSON into one catalog model. Borrowed parsing keeps the hot path tight on large files.",
  },
  {
    label: "Update",
    body: "Merge new messages and combine catalogs, preserving existing translations before anything else.",
  },
  {
    label: "Audit",
    body: "Run release checks across source and target locales and emit structured diagnostics.",
  },
  {
    label: "Compile",
    body: "Emit host-neutral runtime artifacts with stable keys and explicit fallback behavior.",
  },
]

// ── Three explicit storage + semantics combinations ──

const catalogModes = [
  {
    title: "Translator-friendly PO",
    storage: "Gettext PO",
    semantics: "Gettext plurals",
    body: "The catalog shape translation tools already understand, with comments, references, flags, and plural forms preserved.",
  },
  {
    title: "Rich-message PO",
    storage: "Gettext PO",
    semantics: "ICU MessageFormat",
    body: "Keep translator-facing PO files while authoring placeholders, formatting, plurals, selects, and structural diagnostics.",
  },
  {
    title: "Reviewable NDJSON",
    storage: "NDJSON",
    semantics: "ICU MessageFormat",
    body: "One message per line: easier to review, merge, stream, batch, and hand to external systems.",
  },
]

const positioning = [
  {
    before: "Translations live in loose JSON files nobody reviews.",
    after: "Source text, context, plurals, and obsolete messages stay visible and inspectable.",
  },
  {
    before: "Missing or broken strings surface as production bugs.",
    after: "A structured audit fails the build before the release ships.",
  },
  {
    before: "Every framework reimplements catalog logic from scratch.",
    after: "One Rust core that Palamedes and other adapters reuse.",
  },
]

const proof = [
  { value: "60", label: "conformance cases", detail: "derived from upstream gettext" },
  { value: "454", label: "assertions", detail: "checked by the harness" },
  { value: "3", label: "catalog modes", detail: "explicit, no hidden defaults" },
]

// ── Throughput on a 10k-message catalog, release build ──

const perf = [
  { op: "PO parsing", rate: "540 MiB/s", note: "zero-copy borrowed path" },
  { op: "PO serialization", rate: "1.1 GiB/s", note: "direct buffer writes" },
  { op: "Catalog merge", rate: "360 MiB/s", note: "existing translations kept first" },
]

// ── Other open source from the same studio ──

const ossProjects = [
  { name: "universal-dotenv", body: "Robust env config for universal apps", href: "https://github.com/sebastian-software/universal-dotenv" },
  { name: "palamedes", body: "i18n framework powered by OXC", href: "https://github.com/sebastian-software/palamedes" },
  { name: "ardo", body: "React-first static docs framework", href: "https://github.com/sebastian-software/ardo" },
  { name: "pofile-ts", body: "Gettext PO files in TypeScript", href: "https://github.com/sebastian-software/pofile-ts" },
  { name: "effective-color", body: "Perceptual color palette generator", href: "https://github.com/sebastian-software/effective-color" },
  { name: "effective-icon", body: "Compile-time icon pipeline for Vite", href: "https://github.com/sebastian-software/effective-icon" },
  { name: "eslint-config-setup", body: "Flat ESLint configs, zero runtime", href: "https://github.com/sebastian-software/eslint-config-setup" },
  { name: "xlsx-format", body: "Modern XLSX reader and writer", href: "https://github.com/sebastian-software/xlsx-format" },
  { name: "ts-phonenumber", body: "TypeScript-first phone parsing", href: "https://github.com/sebastian-software/ts-phonenumber" },
  { name: "whisper-coreml", body: "Whisper ASR on Apple Silicon", href: "https://github.com/sebastian-software/whisper-coreml" },
  { name: "iktia", body: "Lean TSX compiler", href: "https://github.com/sebastian-software/iktia" },
  { name: "effective-css", body: "Layered CSS reset for evergreen browsers", href: "https://github.com/sebastian-software/effective-css" },
]

export default function HomePage() {
  return (
    <div className="ferro-home">
      {/* ── Hero ── */}
      <section className="ferro-hero">
        <div className="ferro-hero-glow" aria-hidden="true" />
        <p className="ferro-eyebrow ferro-hero-kicker">
          <Hammer size={14} />
          Part of the Ferro family · forged in Rust
        </p>
        <h1 className="ferro-hero-title">
          Make broken translations a build error, not a support ticket.
        </h1>
        <p className="ferro-lead">
          Ferrocat is a Rust-native catalog engine that treats localized copy as
          product data: parse it, merge updates by exact identity, audit every
          locale before release, and compile runtime artifacts your app can ship
          with confidence.
        </p>
        <div className="ferro-hero-cta">
          <pre className="ferro-install">
            <code>cargo add ferrocat</code>
          </pre>
          <div className="ferro-actions">
            <Link
              className="ferro-button ferro-button-primary"
              to="/guide/getting-started"
            >
              Read the guide
              <ArrowRight size={16} />
            </Link>
            <a className="ferro-button ferro-button-tertiary" href={GITHUB}>
              <GitBranch size={16} />
              GitHub
            </a>
            <a
              className="ferro-button ferro-button-tertiary"
              href="https://docs.rs/ferrocat"
            >
              docs.rs
              <ExternalLink size={14} />
            </a>
          </div>
        </div>
      </section>

      {/* ── The Ferro family ── */}
      <section className="ferro-family ferro-reveal">
        <div className="ferro-family-intro">
          <h2>
            <span className="ferro-ferro">Ferro</span> means iron. Every tool
            here is forged in Rust.
          </h2>
          <p className="ferro-sublead">
            Ferrocat is one of six focused tools from the same workshop. Shared
            engineering, shared release discipline, each one replacing a slower
            or heavier dependency in the JavaScript and Rust toolchain.
          </p>
        </div>
        <ul className="ferro-family-grid">
          {family.map((tool) => (
            <li key={tool.name}>
              <a
                className={
                  tool.current
                    ? "ferro-family-card is-current"
                    : "ferro-family-card"
                }
                href={tool.href}
              >
                <span className="ferro-family-icon">{tool.icon}</span>
                <span className="ferro-family-name">
                  {tool.name}
                  {tool.current ? (
                    <span className="ferro-family-tag">you are here</span>
                  ) : null}
                </span>
                <span className="ferro-family-role">{tool.role}</span>
                <span className="ferro-family-body">{tool.body}</span>
              </a>
            </li>
          ))}
        </ul>
      </section>

      {/* ── Problem to outcome ── */}
      <section className="ferro-shift ferro-reveal">
        <div className="ferro-section-heading">
          <h2>Localization that behaves like the rest of your codebase.</h2>
          <p className="ferro-sublead">
            Most projects treat translations as files to load, not data to
            review. Ferrocat moves the catalog into the same discipline as your
            source: explicit identity, real diffs, checks before release.
          </p>
        </div>
        <ul className="ferro-shift-list">
          {positioning.map((row) => (
            <li className="ferro-shift-row" key={row.after}>
              <span className="ferro-shift-before">{row.before}</span>
              <ArrowRight className="ferro-shift-arrow" size={18} />
              <span className="ferro-shift-after">{row.after}</span>
            </li>
          ))}
        </ul>
      </section>

      {/* ── Performance ── */}
      <section className="ferro-perf ferro-reveal">
        <div className="ferro-section-heading">
          <h2>Rust throughput, in a Node-shaped world.</h2>
          <p className="ferro-sublead">
            Most i18n tooling for JavaScript and TypeScript runs on Node, where
            catalog parsing and validation are interpreted. Ferrocat is compiled
            Rust: byte-oriented scanning, zero-copy parsing, and SIMD-accelerated
            escaping on the hot paths.
          </p>
        </div>
        <div className="ferro-perf-board">
          {perf.map((row) => (
            <div className="ferro-perf-row" key={row.op}>
              <span className="ferro-perf-op">{row.op}</span>
              <span className="ferro-perf-rate">{row.rate}</span>
              <span className="ferro-perf-note">{row.note}</span>
            </div>
          ))}
        </div>
        <p className="ferro-perf-foot">
          Release build on a 10k-message catalog.{" "}
          <Link to="/performance/benchmarking">See the methodology</Link>, which
          includes a cross-runtime suite against GNU gettext, Node, and Python
          tooling.
        </p>
      </section>

      {/* ── Benefits ── */}
      <section className="ferro-benefits ferro-reveal">
        <div className="ferro-section-heading">
          <h2>What the engine gives you.</h2>
        </div>
        <div className="ferro-benefit-grid">
          {benefits.map((item) => (
            <article className="ferro-benefit-card" key={item.title}>
              <span className="ferro-benefit-icon">{item.icon}</span>
              <h3>{item.title}</h3>
              <p>{item.body}</p>
            </article>
          ))}
        </div>
      </section>

      {/* ── How it works ── */}
      <section className="ferro-flow ferro-reveal">
        <div className="ferro-section-heading">
          <h2>One catalog, four stages.</h2>
          <p className="ferro-sublead">
            Each stage is a Rust API you can call on its own or chain into a
            release pipeline.
          </p>
        </div>
        <ol className="ferro-flow-steps">
          {steps.map((step, i) => (
            <li className="ferro-flow-step" key={step.label}>
              <span className="ferro-flow-num">{i + 1}</span>
              <h3>{step.label}</h3>
              <p>{step.body}</p>
            </li>
          ))}
        </ol>
      </section>

      {/* ── Catalog modes ── */}
      <section className="ferro-section ferro-reveal">
        <div className="ferro-section-heading">
          <h2>Proven formats, one product workflow.</h2>
          <p className="ferro-sublead">
            Pick the storage and message model per project. Migrations stay
            visible in code instead of hiding in tooling.
          </p>
        </div>
        <div className="ferro-mode-grid">
          {catalogModes.map((mode) => (
            <article className="ferro-mode-card" key={mode.title}>
              <h3>{mode.title}</h3>
              <dl>
                <div>
                  <dt>Storage</dt>
                  <dd>{mode.storage}</dd>
                </div>
                <div>
                  <dt>Semantics</dt>
                  <dd>{mode.semantics}</dd>
                </div>
              </dl>
              <p>{mode.body}</p>
            </article>
          ))}
        </div>
      </section>

      {/* ── Palamedes ── */}
      <section className="ferro-palamedes ferro-reveal">
        <div className="ferro-section-heading">
          <h2>Use it from Rust, or from JS and TypeScript through Palamedes.</h2>
          <p className="ferro-sublead">
            Palamedes is the i18n framework for application teams: macros,
            message extraction, and adapters for Vite and Next.js. Ferrocat is
            the catalog engine beneath it, so JS and TS teams get Rust-speed
            parsing and QA without writing Rust.
          </p>
        </div>
        <div className="ferro-stack">
          <article className="ferro-stack-layer">
            <div className="ferro-stack-head">
              <span className="ferro-stack-name">Palamedes</span>
              <span className="ferro-stack-tag">JS / TS framework</span>
            </div>
            <p>
              Macros, message extraction, framework adapters, and runtime loading
              for application developers.
            </p>
            <a
              className="ferro-way-link"
              href="https://github.com/sebastian-software/palamedes"
            >
              Explore Palamedes
              <ArrowRight size={15} />
            </a>
          </article>
          <div className="ferro-stack-joint" aria-hidden="true">
            <span>built on</span>
          </div>
          <article className="ferro-stack-layer is-engine">
            <div className="ferro-stack-head">
              <span className="ferro-stack-name">Ferrocat</span>
              <span className="ferro-stack-tag">Rust engine</span>
            </div>
            <p>
              Parsing, deterministic updates, release QA, and runtime artifacts.
              Usable directly from Rust or through the ferrocat-cli audit gate.
            </p>
            <Link className="ferro-way-link" to="/guide/palamedes">
              How they fit together
              <ArrowRight size={15} />
            </Link>
          </article>
        </div>
      </section>

      {/* ── Proof ── */}
      <section className="ferro-proof ferro-reveal">
        <div className="ferro-proof-inner">
          <div className="ferro-section-heading">
            <h2>Quality is part of the catalog contract.</h2>
            <p className="ferro-proof-lead">
              Ferrocat's behavior is pinned by conformance fixtures derived from
              upstream gettext and by repeatable benchmark commands, not by hope.
            </p>
          </div>
          <div className="ferro-proof-stats">
            {proof.map((item) => (
              <article className="ferro-proof-stat" key={item.label}>
                <strong>{item.value}</strong>
                <span>{item.label}</span>
                <span className="ferro-proof-detail">{item.detail}</span>
              </article>
            ))}
          </div>
          <div className="ferro-proof-links">
            <Link to="/quality/conformance">
              Conformance snapshot
              <ArrowRight size={16} />
            </Link>
            <Link to="/performance/benchmarking">
              Benchmark methodology
              <ArrowRight size={16} />
            </Link>
            <Link to="/architecture/adr">
              Architecture decisions
              <ArrowRight size={16} />
            </Link>
          </div>
        </div>
      </section>

      {/* ── Sponsor + OSS ── */}
      <section className="ferro-studio ferro-reveal">
        <div className="ferro-studio-head">
          <p className="ferro-eyebrow">Maintained by Sebastian Software</p>
          <h2>An independent studio shipping open source for the long run.</h2>
          <p className="ferro-sublead">
            Ferrocat and the Ferro family are built and maintained by{" "}
            <a className="ferro-inline-link" href={COMPANY}>
              Sebastian Software
            </a>
            . We ship dependable open source for the JavaScript, TypeScript, and
            Rust ecosystems, and we use every one of these tools in production
            ourselves.
          </p>
          <div className="ferro-actions">
            <a className="ferro-button ferro-button-primary" href={OSS}>
              <Boxes size={16} />
              Browse all our open source
            </a>
            <a
              className="ferro-button ferro-button-tertiary"
              href="https://github.com/sponsors/sebastian-software"
            >
              <Flame size={15} />
              Sponsor the work
            </a>
          </div>
        </div>

        <div className="ferro-marquee" aria-label="More open source from Sebastian Software">
          <div className="ferro-marquee-track">
            {[...ossProjects, ...ossProjects].map((p, i) => (
              <a
                className="ferro-oss-card"
                key={`${p.name}-${i}`}
                href={p.href}
                aria-hidden={i >= ossProjects.length ? "true" : undefined}
                tabIndex={i >= ossProjects.length ? -1 : undefined}
              >
                <span className="ferro-oss-name">{p.name}</span>
                <span className="ferro-oss-body">{p.body}</span>
              </a>
            ))}
          </div>
        </div>
      </section>

      {/* ── Final CTA ── */}
      <section className="ferro-cta ferro-reveal">
        <div className="ferro-cta-copy">
          <h2>Put broken translations on the wrong side of your CI.</h2>
          <p className="ferro-sublead">
            Start with one catalog and a single audit call. Add modes, runtime
            artifacts, and AI metadata when you need them.
          </p>
        </div>
        <div className="ferro-actions">
          <Link
            className="ferro-button ferro-button-primary"
            to="/guide/getting-started"
          >
            Read the guide
            <ArrowRight size={16} />
          </Link>
          <a className="ferro-button ferro-button-secondary" href="https://docs.rs/ferrocat">
            docs.rs
            <ExternalLink size={16} />
          </a>
        </div>
      </section>
    </div>
  )
}
