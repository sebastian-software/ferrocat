import {
  ArrowRight,
  BookOpenText,
  Boxes,
  ExternalLink,
  FileStack,
  Gauge,
  Github,
  ShieldCheck,
  Workflow,
} from "lucide-react"
import type { MetaFunction } from "react-router"
import { Link } from "react-router"

export const meta: MetaFunction = () => [
  { title: "Ferrocat Docs" },
  {
    name: "description",
    content:
      "Performance-first translation catalogs for teams that need Gettext compatibility, ICU semantics, and JSON-friendly runtime workflows.",
  },
]

const catalogModes = [
  {
    title: "Classic Gettext catalog mode",
    storage: "Gettext PO",
    semantics: "Gettext-compatible plurals",
    body: "Stay close to traditional gettext catalogs and familiar msgid_plural workflows.",
  },
  {
    title: "ICU-native Gettext PO mode",
    storage: "Gettext PO",
    semantics: "ICU MessageFormat",
    body: "Keep translator-facing PO files while authoring richer ICU plural, select, and formatting messages.",
  },
  {
    title: "ICU-native NDJSON catalog mode",
    storage: "NDJSON catalog storage",
    semantics: "ICU MessageFormat",
    body: "Move runtime-facing and external-tooling workflows to a JSON-friendly line-oriented storage model.",
  },
]

const proofPoints = [
  {
    value: "55 cases",
    label: "upstream-derived conformance cases in the current snapshot",
  },
  {
    value: "442 assertions",
    label: "checked by the conformance harness today",
  },
  {
    value: "3 modes",
    label: "explicit catalog combinations instead of silent fallback behavior",
  },
]

const entryPoints = [
  {
    title: "Get started fast",
    body: "Install the umbrella crate, run the first parse/stringify flow, and learn where the high-level catalog APIs fit.",
    link: "/guide/getting-started",
    icon: <BookOpenText size={18} />,
  },
  {
    title: "Read the API surface",
    body: "Use the practical API overview to choose between PO core, catalog workflows, and ICU helpers.",
    link: "/reference/api-overview",
    icon: <Boxes size={18} />,
  },
  {
    title: "Inspect performance evidence",
    body: "Benchmark methodology, fixtures, and performance history live in one place instead of scattered markdown files.",
    link: "/performance",
    icon: <Gauge size={18} />,
  },
  {
    title: "Trace architecture decisions",
    body: "Accepted ADRs and engineering notes show the reasoning behind semantic choices, hot paths, and compatibility boundaries.",
    link: "/architecture/adr",
    icon: <FileStack size={18} />,
  },
]

export default function HomePage() {
  return (
    <div className="ferro-home">
      <section className="ferro-hero">
        <div className="ferro-hero-copy">
          <p className="ferro-eyebrow">Performance-first localization toolkit</p>
          <h1>Translation catalogs for teams that need Gettext, ICU, and JSON-friendly delivery to coexist cleanly.</h1>
          <p className="ferro-lead">
            Ferrocat brings classic PO workflows, ICU MessageFormat semantics, and runtime-oriented catalog compilation into
            one Rust-native architecture with explicit crate boundaries, conformance evidence, and benchmark discipline.
          </p>
          <div className="ferro-actions">
            <Link className="ferro-button ferro-button-primary" to="/guide/getting-started">
              Start with the guide
              <ArrowRight size={16} />
            </Link>
            <Link className="ferro-button ferro-button-secondary" to="/reference/api-overview">
              Browse the API
              <Workflow size={16} />
            </Link>
            <a className="ferro-button ferro-button-tertiary" href="https://github.com/sebastian-software/ferrocat">
              GitHub
              <Github size={16} />
            </a>
          </div>
        </div>
        <div className="ferro-hero-aside">
          <div className="ferro-panel">
            <p className="ferro-panel-label">Install</p>
            <pre className="ferro-code">
              <code>cargo add ferrocat</code>
            </pre>
          </div>
          <div className="ferro-panel ferro-panel-muted">
            <p className="ferro-panel-label">What Ferrocat optimizes for</p>
            <ul className="ferro-checklist">
              <li>Rust-native hot paths instead of translated legacy abstractions</li>
              <li>Explicit storage and semantics modes that are easy to reason about</li>
              <li>Runtime compilation APIs for downstream adapters and bundlers</li>
            </ul>
          </div>
        </div>
      </section>

      <section className="ferro-section">
        <div className="ferro-section-heading">
          <p className="ferro-eyebrow">Why Ferrocat exists</p>
          <h2>Most teams are forced to choose between workflow compatibility and a modern runtime story.</h2>
        </div>
        <div className="ferro-grid ferro-grid-two">
          <article className="ferro-card">
            <h3>Keep the real translation workflow</h3>
            <p>
              Translator comments, references, contexts, and Gettext-shaped catalogs still matter in production systems.
              Ferrocat keeps that reality visible instead of pretending localization is a flat key-value problem.
            </p>
          </article>
          <article className="ferro-card">
            <h3>Upgrade semantics and delivery deliberately</h3>
            <p>
              Teams can stay in classic PO mode, adopt ICU-native messages inside PO files, or move to NDJSON when external
              systems want a line-oriented JSON representation.
            </p>
          </article>
        </div>
      </section>

      <section className="ferro-section">
        <div className="ferro-section-heading">
          <p className="ferro-eyebrow">Three catalog modes</p>
          <h2>Storage and message semantics stay explicit.</h2>
        </div>
        <div className="ferro-mode-grid">
          {catalogModes.map((mode) => (
            <article className="ferro-card ferro-mode-card" key={mode.title}>
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
        <p className="ferro-note">
          There is intentionally no NDJSON + gettext-compatible plurals mode. The guide explains the boundaries in detail.
        </p>
      </section>

      <section className="ferro-section ferro-proof">
        <div className="ferro-section-heading">
          <p className="ferro-eyebrow">Proof, not vibes</p>
          <h2>Performance, conformance, and architecture quality are documented as first-class product surfaces.</h2>
        </div>
        <div className="ferro-proof-grid">
          {proofPoints.map((item) => (
            <article className="ferro-proof-card" key={item.label}>
              <strong>{item.value}</strong>
              <p>{item.label}</p>
            </article>
          ))}
        </div>
        <div className="ferro-grid ferro-grid-two">
          <article className="ferro-card">
            <ShieldCheck size={18} />
            <h3>Trust the compatibility story</h3>
            <p>
              The conformance snapshot ties behavior back to upstream ecosystems rather than vague “close enough”
              compatibility claims.
            </p>
            <Link to="/quality/conformance">Read conformance notes</Link>
          </article>
          <article className="ferro-card">
            <Gauge size={18} />
            <h3>Read the benchmark methodology</h3>
            <p>
              Benchmark fixtures, official profiles, and performance history are organized as a coherent documentation
              trail instead of being buried in one long README.
            </p>
            <Link to="/performance/benchmarking">See performance docs</Link>
          </article>
        </div>
      </section>

      <section className="ferro-section">
        <div className="ferro-section-heading">
          <p className="ferro-eyebrow">Choose a path</p>
          <h2>Start from the question you actually have.</h2>
        </div>
        <div className="ferro-entry-grid">
          {entryPoints.map((entry) => (
            <Link className="ferro-entry-card" key={entry.title} to={entry.link}>
              <span className="ferro-entry-icon">{entry.icon}</span>
              <h3>{entry.title}</h3>
              <p>{entry.body}</p>
              <span className="ferro-entry-link">
                Open section
                <ArrowRight size={16} />
              </span>
            </Link>
          ))}
        </div>
      </section>

      <section className="ferro-cta">
        <div>
          <p className="ferro-eyebrow">Ready to dig in</p>
          <h2>Use the curated docs site for the full story, and keep docs.rs plus GitHub close for day-to-day development.</h2>
        </div>
        <div className="ferro-actions">
          <a className="ferro-button ferro-button-primary" href="https://docs.rs/ferrocat">
            docs.rs
            <ExternalLink size={16} />
          </a>
          <Link className="ferro-button ferro-button-secondary" to="/guide/getting-started">
            Installation and quick start
            <ArrowRight size={16} />
          </Link>
        </div>
      </section>
    </div>
  )
}
