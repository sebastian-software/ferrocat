import { ArdoLink as Link } from "ardo/runtime";
import {
  FerraAction,
  FerraArticle,
  FerraChoices,
  FerraCode,
  FerraFeatureGrid,
  FerraSteps,
} from "ferramenta-family/ferra";
import { family, WORKSHOP } from "ferramenta-family/registry";

export function CatalogSection() {
  return (
    <FerraArticle
      id="catalogs"
      title="Translator-friendly files. Git-friendly changes."
      introduction="PO connects to the translation tools your team already knows. FCL gives machine-managed ICU catalogs a line of their own."
      meta={<Link to="/guide/catalog-modes">Choose your catalog mode</Link>}
    >
      <p>
        Ferrocat Catalog Lines puts each message on one deterministically ordered line. Independent
        changes stay separate in a Git diff, and an entry cannot be split across several lines
        during a merge. When two branches edit the same entry, the conflict stays visible.
      </p>
      <FerraCode caption="FCL · one tab-separated record per message" file="de.fcl">
        {
          "%FCL1\tsource=en\tlocale=de\torder=collated\nHello {name}\tgreeting\tHallo {name}\nPay now\tcheckout\tJetzt bezahlen\n"
        }
      </FerraCode>
      <p>
        FCL is written by tools and uses ICU MessageFormat semantics. Keep classic PO for Gettext
        plurals, or ICU-native PO when external translation tools need PO files. You choose the mode
        explicitly; Ferrocat keeps storage and message semantics together.
      </p>
    </FerraArticle>
  );
}
export function ReviewSection() {
  return (
    <FerraArticle
      id="review"
      title="A human correction should leave a trace."
      introduction="Machine translation can start the work. A review report helps you see what happened next."
      meta={
        <Link to="/reference/api-overview#machine-managed-value-metadata">
          Integrity locks and AI provenance
        </Link>
      }
    >
      <FerraSteps
        caption="From a machine-written value to an informed review"
        items={[
          {
            heading: "Record the value",
            text: "An integrity lock fingerprints the machine-written translation. Optional metadata records the model and confidence.",
          },
          {
            heading: "Keep the human edit visible",
            text: "If someone improves the translation, its value no longer matches the stored lock.",
          },
          {
            heading: "Choose what happens next",
            text: "Your application can use that evidence in its review or retranslation policy. Ferrocat exposes the intervention rather than guessing your policy.",
          },
        ]}
      />
      <p>
        Review reports also distinguish new messages, removed messages and changed translations.
        Share a catalog-state comparison before a translator handoff, or use it to make an automated
        update explainable in a pull request.
      </p>
    </FerraArticle>
  );
}
export function CapabilitiesSection() {
  return (
    <FerraArticle
      title="From a catalog file to something your app can use."
      meta={<Link to="/reference/api-overview">Explore the Rust API</Link>}
    >
      <FerraFeatureGrid
        items={[
          {
            heading: "Updates with a clear identity",
            text: "Merge newly extracted messages by exact identity, preserving existing translations. No fuzzy ID changes or silent conflict resolution.",
          },
          {
            heading: "Rich messages, checked",
            text: "Inspect ICU placeholders, plurals, selects and tags. Compare source and translation structure before compiling.",
          },
          {
            heading: "Runtime delivery",
            text: "Compile locale-resolved payloads with stable keys, explicit fallback, missing-message records and provenance for host tools.",
          },
          {
            heading: "Pseudo-locales that keep working",
            text: "Expand and transform display text while preserving ICU placeholders, selectors, formatters and rich-text tags.",
          },
        ]}
      />
    </FerraArticle>
  );
}
export function GettingStartedSection() {
  return (
    <FerraChoices>
      <FerraArticle
        id="start"
        title="Build with the Rust core."
        introduction="Add the umbrella crate to your application. Start with PO parsing, then reach for catalog updates, review and compilation as you need them."
      >
        <FerraCode caption="Add the library">cargo add ferrocat</FerraCode>
        <FerraAction href="/guide/getting-started">Start with Rust</FerraAction>
      </FerraArticle>
      <FerraArticle
        title="Make catalog audits a release check."
        introduction="Install the CLI, point it at your source and target locales, and let its exit status carry the result into CI."
      >
        <FerraCode caption="Install the audit CLI">cargo install ferrocat-cli</FerraCode>
        <FerraAction variant="secondary" href="/reference/api-overview#audit-catalogs">
          Set up an audit
        </FerraAction>
      </FerraArticle>
    </FerraChoices>
  );
}
export function ClosingSection() {
  const palamedes = family.find((entry) => entry.name === "palamedes");
  return (
    <FerraArticle
      title="One focused engine. Your application around it."
      meta={<a href={WORKSHOP.openSource}>More open source from {WORKSHOP.company}</a>}
    >
      <p>
        Ferrocat owns catalog semantics in Rust.{" "}
        {palamedes && <a href={palamedes.docs}>Palamedes</a>} owns JavaScript and TypeScript
        extraction, macros, framework adapters and bindings. You can use the Rust crate or CLI
        independently, and keep application-specific choices in your host tool.
      </p>
      <p>
        <Link to="/guide/palamedes">How the responsibilities fit together</Link>
        {" · "}
        <a href="https://ferramenta.dev">Meet the Ferramenta family</a>
      </p>
    </FerraArticle>
  );
}
