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
      "Parse, update, review, audit, and compile translations with a Rust-native catalog engine. Missing strings, ICU mistakes, stale copy, and weak coverage can become CI diagnostics before they reach production.",
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
    body: "PO, FCL, and ICU MessageFormat with merge, review, and audit. Parses several times faster than Node catalog tooling, and merges faster still.",
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
    body: "Keep source text, context, plurals, notes, source origins, and obsolete entries in a single model your application code can reason about.",
    icon: <Boxes size={20} />,
  },
  {
    title: "Updates without guesswork",
    body: "Merge freshly extracted messages into existing catalogs by exact identity. No fuzzy matching, no hidden ID changes, no silent conflict resolution.",
    icon: <GitMerge size={20} />,
  },
  {
    title: "Release QA with numbers",
    body: "Audit for missing locales, empty translations, stale targets, ICU errors, metadata conflicts, and obsolete entries. Coverage reports make the gap visible before release day.",
    icon: <ShieldCheck size={20} />,
  },
  {
    title: "Review reports for handoffs",
    body: "Compare catalog states before a translator handoff. New strings, removed strings, changed translations, and which machine translations a human has edited since: all in one report.",
    icon: <BookOpenText size={20} />,
  },
  {
    title: "Rich messages that keep their values",
    body: "Analyze placeholders, formatters, plurals, selects, and tags. Runtime-specific formatter support stays explicit, so unsupported message shapes fail before they ship.",
    icon: <ScanSearch size={20} />,
  },
  {
    title: "Runtime artifacts you can explain",
    body: "Compile locale-resolved payloads with stable keys, explicit fallback, missing-message records, and provenance rows for host tools that need to show where a string came from.",
    icon: <Package size={20} />,
  },
  {
    title: "Pseudo-locales without broken ICU",
    body: "Pseudolocalize final ICU messages and compiled artifacts while preserving placeholders, plural selectors, formatter syntax, and rich-text tags.",
    icon: <Languages size={20} />,
  },
  {
    title: "AI-native metadata that stays honest",
    body: "Tag any machine-written value (AI model, TMS, or a script) with an integrity lock plus optional model and confidence. Ship it as-is by default; when a human corrects one, the lock stops matching, so your next re-translation run won't silently overwrite their fix.",
    icon: <FileCheck2 size={20} />,
  },
]

// ── The path a catalog takes through Ferrocat ──

const steps = [
  {
    label: "Parse",
    body: "Read PO or FCL into one catalog model. Borrowed parsing keeps the hot path tight on large files.",
  },
  {
    label: "Update",
    body: "Merge new messages and combine catalogs, preserving existing translations before anything else.",
  },
  {
    label: "Review",
    body: "Summarize coverage and catalog-state changes before translator handoff or CI thresholds.",
  },
  {
    label: "Audit",
    body: "Run release checks across source and target locales with diagnostics CI can read.",
  },
  {
    label: "Compile",
    body: "Emit host-neutral runtime artifacts, provenance reports, and pseudo-locale variants.",
  },
]

// ── Three explicit storage + semantics combinations ──

const catalogModes = [
  {
    title: "Translator-friendly PO",
    storage: "Gettext PO",
    semantics: "Gettext plurals",
    body: "The catalog shape translation tools already understand, with comments, references, and plural forms preserved.",
  },
  {
    title: "Rich-message PO",
    storage: "Gettext PO",
    semantics: "ICU MessageFormat",
    body: "Keep translator-facing PO files while authoring placeholders, formatting, plurals, selects, and structural diagnostics.",
  },
  {
    title: "Mergeable FCL",
    storage: "FCL",
    semantics: "ICU MessageFormat",
    body: "Ordinary git merges stop losing untouched translations: one canonical, sorted entry per line means only real edits collide. It also parses ~45% faster and stores ~12% smaller than the same catalog as PO.",
  },
]

const positioning = [
  {
    before: "Translations live in loose JSON files nobody reviews.",
    after: "Source text, context, plurals, obsolete messages, and coverage gaps stay visible.",
  },
  {
    before: "Missing or broken strings surface as production bugs.",
    after: "A structured audit fails the build before the release ships.",
  },
  {
    before: "Translator handoffs depend on ad hoc diff reading.",
    after: "Catalog review reports show what changed and which locales need work.",
  },
  {
    before: "Every framework reimplements catalog logic from scratch.",
    after: "One Rust core that Palamedes and other adapters reuse.",
  },
]

const proof = [
  { value: "60+", label: "conformance cases", detail: "derived from upstream gettext" },
  { value: "95%+", label: "library coverage gate", detail: "for the main catalog crates" },
]

// ── Cross-runtime throughput, gettext-official + workflows profiles ──
// Same 10k-message gettext catalog, median MiB/s on an Apple M1 Ultra.
// Every tool reads the same files; see benchmark/results for the report.

type Bar = { name: string; lang: string; rate: number; self?: boolean }

const parseCompare: Bar[] = [
  { name: "Ferrocat", lang: "Rust, zero-copy", rate: 695, self: true },
  { name: "pofile-ts", lang: "Node", rate: 148 },
  { name: "gettext/gettext", lang: "PHP", rate: 49 },
  { name: "polib", lang: "Python", rate: 20 },
]
const PARSE_MAX = 695

const updateCompare: Bar[] = [
  { name: "Ferrocat", lang: "Rust, plain update", rate: 255, self: true },
  {
    name: "Ferrocat full catalog update",
    lang: "with ICU checks + placeholder tracking",
    rate: 103,
    self: true,
  },
  { name: "pofile-ts", lang: "Node", rate: 42 },
  { name: "gettext/gettext", lang: "PHP", rate: 16 },
  { name: "Babel", lang: "Python, Catalog.update", rate: 7.9 },
  { name: "polib", lang: "Python", rate: 7.4 },
  { name: "msgmerge", lang: "GNU gettext", rate: 4.6 },
]
const UPDATE_MAX = 255

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
          Ferrocat is a Rust-native catalog engine that parses and merges
          translation catalogs several times faster than the Node tooling most
          JS and TS teams run today, then treats that copy as product data you
          can review, audit, and compile for production. Catalog problems stay
          in CI, where they belong.
        </p>
        <p className="ferro-hero-craft">
          Hand-tuned SIMD and zero-copy scanning under the hood. The kind of
          parser you don&rsquo;t write in an afternoon.
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
              Run the quick start
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
          <h2>
            Several times faster than Node. An order of magnitude past PHP and
            Python.
          </h2>
          <p className="ferro-sublead">
            V8 is not a slow target. Node&rsquo;s JIT is one of the fastest
            dynamic runtimes ever shipped, which is exactly why most JS and TS
            i18n tooling feels fine until you put it next to compiled Rust. On
            the same 10k-message catalog, reading the same files, Ferrocat still
            parses several times faster than the quickest Node parser&mdash;and
            the PHP and Python stacks, with no JIT that helps here, fall much
            further back. The part we like most: Ferrocat&rsquo;s{" "}
            <em>full</em> catalog update&mdash;ICU structure analysis,
            placeholder tracking, deterministic output, work none of the other
            tools even attempt&mdash;still finishes twice as fast as the
            quickest plain merge in the field.
          </p>
        </div>
        <div className="ferro-bar-charts">
          <figure className="ferro-bars">
            <figcaption>Parsing a catalog · MiB/s · higher is better</figcaption>
            {parseCompare.map((bar) => (
              <div
                className={bar.self ? "ferro-bar is-self" : "ferro-bar"}
                key={bar.name}
              >
                <span className="ferro-bar-label">
                  {bar.name}
                  <span className="ferro-bar-lang">{bar.lang}</span>
                </span>
                <span className="ferro-bar-track">
                  <span
                    className="ferro-bar-fill"
                    style={{ width: `${(bar.rate / PARSE_MAX) * 100}%` }}
                  />
                </span>
                <span className="ferro-bar-value">{bar.rate}</span>
              </div>
            ))}
          </figure>
          <figure className="ferro-bars">
            <figcaption>
              Updating with new strings · MiB/s · the release-time job
            </figcaption>
            {updateCompare.map((bar) => (
              <div
                className={bar.self ? "ferro-bar is-self" : "ferro-bar"}
                key={bar.name}
              >
                <span className="ferro-bar-label">
                  {bar.name}
                  <span className="ferro-bar-lang">{bar.lang}</span>
                </span>
                <span className="ferro-bar-track">
                  <span
                    className="ferro-bar-fill"
                    style={{ width: `${(bar.rate / UPDATE_MAX) * 100}%` }}
                  />
                </span>
                <span className="ferro-bar-value">{bar.rate}</span>
              </div>
            ))}
          </figure>
        </div>
        <p className="ferro-perf-craft">
          None of this comes free with picking Rust. The hot path is written by
          hand: memchr scanning, NEON SIMD on Apple Silicon, borrowed parsing
          that never copies the source, and a merge that moves data instead of
          cloning it. Months of low-level work you inherit the moment you add the
          crate.
        </p>
        <p className="ferro-perf-foot">
          Parsing is mostly raw scanning, and a warm JIT is genuinely good at
          raw scanning, so the parse gap is the narrowest one on this page. The
          honest part is admitting that. Updating is the real release-time job:
          parse the existing catalog, parse the freshly extracted strings, merge
          by identity, and write it back. Once allocation and serialization
          dominate, the JIT&rsquo;s edge fades and the zero-copy,
          move-not-clone hot path pulls further ahead&mdash;which is why the
          update lead is wider than the parse lead. The second Ferrocat bar is
          the high-level catalog update on the same files: on top of the plain
          update it analyzes ICU message structure, tracks placeholders, and
          produces deterministic output. None of the compared tools has an
          equivalent layer&mdash;their update <em>is</em> the plain bar. The
          broader benchmark now includes Babel&rsquo;s real Python
          <code>Catalog.update</code> path, and a compatibility probe asserts
          the output stays semantically identical to msgmerge&rsquo;s on this
          corpus. The GNU msgmerge bar is not
          a launch-cost artifact either: the benchmark records an empty-run
          baseline, and its fixed process and I/O overhead is about 2% of the
          measured time on this corpus, so the gap is real work. The Node
          baseline, pofile-ts, is our own performance fork of the popular
          pofile&mdash;so the fastest JS parser here is one we built, and Rust
          still leads it ~4.7x; the unforked original sits about 320x back. The
          parse chart uses
          borrowed, zero-copy parsing; reading into a fully owned model still
          reaches about 510 MiB/s. Serialization runs at about 1.4 GiB/s on the same
          corpus. Median throughput on an Apple M1 Ultra, every tool reading
          the same files (pofile-ts 4.0.3, gettext-parser 9.0.2, Babel 2.18.0,
          polib 1.2.0, gettext/gettext 5.7.3, GNU gettext 1.0).{" "}
          <Link to="/performance/benchmarking">Methodology</Link> and{" "}
          <a href="https://github.com/sebastian-software/ferrocat/tree/main/benchmark/results">
            full report
          </a>
          .
        </p>
      </section>

      {/* ── Benefits ── */}
      <section className="ferro-benefits ferro-reveal">
        <div className="ferro-section-heading">
          <h2>Everything around the catalog file.</h2>
          <p className="ferro-sublead">
            A parser is only the start. The harder work is what happens around
            it: handoffs, coverage thresholds, runtime provenance,
            pseudo-locales, and checks your release process can trust.
          </p>
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
          <h2>One catalog, five jobs.</h2>
          <p className="ferro-sublead">
            Each job is a Rust API you can call on its own or chain into a
            release pipeline. Start small, then add the checks that match your
            risk.
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
              Parsing, deterministic updates, review reports, release QA,
              runtime artifacts, and pseudo-locale output. Usable directly from
              Rust or through the ferrocat-cli audit gate.
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
              upstream gettext, crate-level coverage gates, and benchmark
              regression checks that run on pull requests.
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
            <Link to="/quality/test-coverage">
              Coverage policy
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
            Start with one catalog and a single audit call. Add coverage,
            review reports, runtime artifacts, pseudo-locales, and AI metadata
            when the workflow needs them.
          </p>
        </div>
        <div className="ferro-actions">
          <Link
            className="ferro-button ferro-button-primary"
            to="/guide/getting-started"
          >
            Run the quick start
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
