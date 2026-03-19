import {
  ArrowRight,
  BookOpenText,
  Boxes,
  ExternalLink,
  FileStack,
  Gauge,
  Github,
} from "lucide-react"
import type { MetaFunction } from "react-router"
import { Link } from "react-router"

export const meta: MetaFunction = () => [
  { title: "Ferrocat — Performance-first translation catalogs" },
  {
    name: "description",
    content:
      "Rust-native translation catalogs for teams that need Gettext compatibility, ICU semantics, and JSON-friendly runtime workflows. Benchmarked, not hand-waved.",
  },
]

const perfPillars = [
  {
    title: "Byte-oriented scanning",
    body: "PO parsing operates directly on byte sequences. No intermediate string allocations on the critical path.",
  },
  {
    title: "Borrowed & owned APIs",
    body: "Zero-copy borrowed access for read-heavy workloads. Owned structures when you need to mutate.",
  },
  {
    title: "Profiling-driven iteration",
    body: "Performance claims backed by real profiling sessions and published benchmark fixtures — not guesswork.",
  },
]

const catalogModes = [
  {
    title: "Classic Gettext",
    storage: "Gettext PO",
    semantics: "Gettext-compatible plurals",
    body: "Stay close to traditional gettext catalogs and familiar msgid_plural workflows.",
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
    body: "JSON-friendly line-oriented storage for external systems and modern toolchains.",
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
    body: "Install, parse, serialize. Learn where the high-level catalog APIs fit.",
    link: "/guide/getting-started",
    icon: <BookOpenText size={20} />,
  },
  {
    title: "API surface",
    body: "Choose between PO core, catalog workflows, and ICU helpers.",
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
    title: "Architecture",
    body: "ADRs and engineering notes behind semantic choices and hot paths.",
    link: "/architecture/adr",
    icon: <FileStack size={20} />,
  },
]

export default function HomePage() {
  return (
    <div className="ferro-home">
      <section className="ferro-hero">
        <p className="ferro-eyebrow">Rust-native localization toolkit</p>
        <h1>Your localization layer shouldn't be the&nbsp;bottleneck.</h1>
        <p className="ferro-lead">
          Ferrocat brings Gettext PO workflows, ICU MessageFormat semantics,
          and JSON delivery into one Rust-native toolkit — with conformance
          evidence, benchmark discipline, and explicit architecture boundaries.
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
            <Github size={16} />
          </a>
        </div>
      </section>

      <section className="ferro-perf">
        <div className="ferro-section-heading">
          <p className="ferro-eyebrow">Why it's fast</p>
          <h2>Speed from structure, not&nbsp;shortcuts.</h2>
        </div>
        <p className="ferro-sublead">
          Every hot path in Ferrocat is Rust-native, byte-oriented, and shaped
          by profiling — not by porting legacy abstractions into a faster
          language.
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
            <p className="ferro-eyebrow">Proof, not vibes</p>
            <h2>
              Conformance and performance are part of the
              product&nbsp;surface.
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
        <h2>Ready to dig&nbsp;in?</h2>
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
