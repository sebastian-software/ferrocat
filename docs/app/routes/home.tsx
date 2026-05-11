import {
  ArrowRight,
  BookOpenText,
  Boxes,
  ExternalLink,
  FileStack,
  Gauge,
  GitBranch,
  Layers3,
} from "lucide-react"
import type { MetaFunction } from "react-router"
import { Link } from "react-router"

export const meta: MetaFunction = () => [
  { title: "Ferrocat - Rust-native translation catalogs" },
  {
    name: "description",
    content:
      "Rust-native translation catalogs for teams that need PO workflows, ICU semantics, JSON-friendly runtime delivery, and measured performance.",
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
    title: "Classic Gettext",
    storage: "Gettext PO",
    semantics: "Gettext-compatible plurals",
    body: "Stay close to traditional PO catalogs and familiar msgid_plural workflows.",
  },
  {
    title: "ICU-native PO",
    storage: "Gettext PO",
    semantics: "ICU MessageFormat",
    body: "Keep translator-facing PO files while authoring richer ICU plural, select, and formatting messages.",
  },
  {
    title: "ICU-native NDJSON",
    storage: "NDJSON",
    semantics: "ICU MessageFormat",
    body: "Use one-message-per-line JSON records that are easier to review, merge, stream, and batch.",
  },
]

const proofPoints = [
  { value: "55", label: "conformance cases", detail: "upstream-derived" },
  { value: "442", label: "assertions", detail: "checked by harness" },
  { value: "3", label: "catalog modes", detail: "explicit combinations" },
]

const entryPoints = [
  {
    title: "Get started",
    body: "Install Ferrocat, parse your first PO file, and find the right catalog workflow.",
    link: "/guide/getting-started",
    icon: <BookOpenText size={20} />,
  },
  {
    title: "API surface",
    body: "Choose between PO core APIs, high-level catalog workflows, and ICU helpers.",
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
    body: "See how Ferrocat powers the catalog layer for the JS and TS i18n framework.",
    link: "/guide/palamedes",
    icon: <Layers3 size={20} />,
  },
  {
    title: "Architecture",
    body: "ADRs and engineering notes behind semantic choices, storage modes, and hot paths.",
    link: "/architecture/adr",
    icon: <FileStack size={20} />,
  },
]

export default function HomePage() {
  return (
    <div className="ferro-home">
      <section className="ferro-hero">
        <p className="ferro-eyebrow">Rust-native translation catalogs</p>
        <h1>Localization catalogs that keep up with your product.</h1>
        <p className="ferro-lead">
          Ferrocat gives Rust teams real PO workflows, explicit ICU and Gettext
          semantics, deterministic catalog updates, and runtime artifacts that
          are fast to load.
        </p>
        <pre className="ferro-install">
          <code>cargo add ferrocat</code>
        </pre>
        <div className="ferro-actions">
          <Link
            className="ferro-button ferro-button-primary"
            to="/guide/getting-started"
          >
            Get started
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

      <section className="ferro-perf">
        <div className="ferro-section-heading">
          <p className="ferro-eyebrow">Performance</p>
          <h2>Fast paths are designed, measured, and&nbsp;documented.</h2>
        </div>
        <p className="ferro-sublead">
          Ferrocat avoids treating PO files as tiny configuration blobs. Large
          catalogs get byte-oriented parsing, borrowed read paths, dedicated
          runtime compilation, and benchmark coverage.
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
          <p className="ferro-eyebrow">Three catalog modes</p>
          <h2>Storage and semantics stay&nbsp;explicit.</h2>
        </div>
        <p className="ferro-sublead">
          Ferrocat is the catalog layer Palamedes can build on: PO when you need
          translator tooling, NDJSON when large-team Git workflows need cleaner
          diffs, and compiled artifacts when applications need fast runtime
          loading.
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
              Compatibility and performance are treated as product behavior.
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
            Get started
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
