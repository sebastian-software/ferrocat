import {
  ArrowRight,
  BookOpenText,
  Boxes,
  ExternalLink,
  FileStack,
  Gauge,
  GitBranch,
  Layers3,
  ShieldCheck,
} from "lucide-react"
import type { MetaFunction } from "react-router"
import { Link } from "react-router"

export const meta: MetaFunction = () => [
  { title: "Ferrocat - product-grade translation catalogs" },
  {
    name: "description",
    content:
      "A Rust-native translation catalog engine for teams that need reviewable copy, release QA, runtime artifacts, and a solid foundation under app localization.",
  },
]

const perfPillars = [
  {
    title: "Byte-oriented scanning",
    body: "PO parsing works directly on byte sequences, keeping the hot path tight for large catalogs.",
  },
  {
    title: "Borrowed and owned APIs",
    body: "Read-heavy paths can borrow from the input buffer. Mutation flows use owned catalog structures.",
  },
  {
    title: "Benchmark fixtures",
    body: "Parser, serializer, merge, combine, and runtime paths are measured against repeatable fixtures.",
  },
]

const catalogModes = [
  {
    title: "Translator-friendly PO",
    storage: "Gettext PO",
    semantics: "Gettext-compatible plurals",
    body: "Use the catalog shape many translation tools already understand, with comments, references, flags, and plural forms preserved.",
  },
  {
    title: "Rich-message PO",
    storage: "Gettext PO",
    semantics: "ICU MessageFormat",
    body: "Keep translator-facing PO files while authoring messages with placeholders, formatting, plurals, selects, and structural diagnostics.",
  },
  {
    title: "Reviewable NDJSON",
    storage: "NDJSON",
    semantics: "ICU MessageFormat",
    body: "Use one-message-per-line JSON records that are easier to review, merge, stream, batch, and hand to external systems.",
  },
]

const positioningPoints = [
  {
    title: "Treat copy as product data",
    body: "Source text, translator context, placeholders, tags, plural logic, and obsolete messages stay visible instead of disappearing into loose files.",
  },
  {
    title: "Know what is safe to ship",
    body: "Run structured audits for missing translations, empty strings, stale target entries, ICU drift, metadata conflicts, and fuzzy flags before release.",
  },
  {
    title: "Stay ready for AI translation",
    body: "Keep machine-generated entries traceable with model, confidence, modified time, and a hash that clears stale metadata after human edits.",
  },
  {
    title: "Keep runtime delivery predictable",
    body: "Compile locale-resolved artifacts with stable keys and explicit fallback behavior instead of reparsing translator files in production.",
  },
]

const proofPoints = [
  { value: "60", label: "conformance cases", detail: "upstream-derived" },
  { value: "454", label: "assertions", detail: "checked by harness" },
  { value: "3", label: "catalog modes", detail: "explicit combinations" },
]

const entryPoints = [
  {
    title: "Get started",
    body: "Install Ferrocat, parse your first catalog, and see the main product workflows.",
    link: "/guide/getting-started",
    icon: <BookOpenText size={20} />,
  },
  {
    title: "API surface",
    body: "Choose between parsing, catalog updates, audit reports, runtime compilation, and rich-message diagnostics.",
    link: "/reference/api-overview",
    icon: <Boxes size={20} />,
  },
  {
    title: "Performance",
    body: "Benchmark methodology, fixtures, and performance history.",
    link: "/performance",
    icon: <Gauge size={20} />,
  },
  {
    title: "Palamedes",
    body: "See how the JS and TS framework can use Ferrocat as its shared catalog engine.",
    link: "/guide/palamedes",
    icon: <Layers3 size={20} />,
  },
  {
    title: "Architecture",
    body: "ADRs and engineering notes behind semantic choices, storage modes, and hot paths.",
    link: "/architecture/adr",
    icon: <FileStack size={20} />,
  },
  {
    title: "Catalog QA",
    body: "Audit completeness, stale entries, ICU drift, metadata conflicts, obsolete entries, and visible fuzzy flags.",
    link: "/reference/api-overview#audit_catalogs",
    icon: <ShieldCheck size={20} />,
  },
]

export default function HomePage() {
  return (
    <div className="ferro-home">
      <section className="ferro-hero">
        <p className="ferro-eyebrow">Product-grade translation catalogs</p>
        <h1>Make translations part of your release process.</h1>
        <p className="ferro-lead">
          Ferrocat is a Rust-native catalog engine for teams that want localized
          copy to be reviewable, testable, and ready for runtime delivery. It
          keeps source text, translator context, validation, QA, and compiled
          payloads in one inspectable layer.
        </p>
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
          <a
            className="ferro-button ferro-button-tertiary"
            href="https://github.com/sebastian-software/ferrocat"
          >
            GitHub
            <GitBranch size={16} />
          </a>
        </div>
      </section>

      <section className="ferro-positioning">
        <div className="ferro-section-heading">
          <p className="ferro-eyebrow">Why Ferrocat</p>
          <h2>
            A catalog engine, not another pile of translation files.
          </h2>
        </div>
        <div className="ferro-positioning-grid">
          {positioningPoints.map((point) => (
            <article className="ferro-positioning-card" key={point.title}>
              <h3>{point.title}</h3>
              <p>{point.body}</p>
            </article>
          ))}
        </div>
      </section>

      <section className="ferro-perf">
        <div className="ferro-section-heading">
          <p className="ferro-eyebrow">Performance</p>
          <h2>Fast paths are designed, measured, and&nbsp;documented.</h2>
        </div>
        <p className="ferro-sublead">
          Ferrocat is designed for catalogs that grow with real products. Large
          translation sets get byte-oriented parsing, borrowed read paths,
          deterministic updates, structured diagnostics, runtime compilation,
          and benchmark coverage.
        </p>
        <div className="ferro-perf-grid">
          {perfPillars.map((pillar) => (
            <article className="ferro-perf-card" key={pillar.title}>
              <h3>{pillar.title}</h3>
              <p>{pillar.body}</p>
            </article>
          ))}
        </div>
        <Link className="ferro-section-link" to="/performance/benchmarking">
          Read the benchmark methodology
          <ArrowRight size={16} />
        </Link>
      </section>

      <section className="ferro-section">
        <div className="ferro-section-heading">
          <p className="ferro-eyebrow">Standards where they help</p>
          <h2>Proven formats, one product workflow.</h2>
        </div>
        <p className="ferro-sublead">
          Ferrocat presents PO, ICU MessageFormat, and NDJSON as explicit choices
          instead of hidden implementation details. Use translator-friendly files
          when people need editing context, line-oriented records when teams need
          clean diffs, rich-message semantics when copy needs runtime values, and
          compiled artifacts when applications need fast loading.
        </p>
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

      <section className="ferro-proof">
        <div className="ferro-proof-inner">
          <div className="ferro-section-heading">
            <p className="ferro-eyebrow">Evidence</p>
            <h2>
              Quality is part of the catalog contract.
            </h2>
          </div>
          <div className="ferro-proof-stats">
            {proofPoints.map((item) => (
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

      <section className="ferro-section">
        <div className="ferro-section-heading">
          <p className="ferro-eyebrow">Start here</p>
          <h2>Pick the path that matches your&nbsp;question.</h2>
        </div>
        <div className="ferro-entry-grid">
          {entryPoints.map((entry) => (
            <Link
              className="ferro-entry-card"
              key={entry.title}
              to={entry.link}
            >
              <span className="ferro-entry-icon">{entry.icon}</span>
              <h3>{entry.title}</h3>
              <p>{entry.body}</p>
              <span className="ferro-entry-link">
                Open
                <ArrowRight size={16} />
              </span>
            </Link>
          ))}
        </div>
      </section>

      <section className="ferro-cta">
        <h2>Build with catalog behavior you can inspect.</h2>
        <div className="ferro-actions">
          <Link
            className="ferro-button ferro-button-primary"
            to="/guide/getting-started"
          >
            Read the guide
            <ArrowRight size={16} />
          </Link>
          <a
            className="ferro-button ferro-button-secondary"
            href="https://docs.rs/ferrocat"
          >
            docs.rs
            <ExternalLink size={16} />
          </a>
        </div>
      </section>
    </div>
  )
}
