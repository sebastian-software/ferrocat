import type { ArdoMetaFunction } from "ardo/runtime";

import { ArdoLink as Link } from "ardo/runtime";
import { Mark, RegistryFacts } from "ferramenta-family";
import {
  FerraAction,
  FerraArticle,
  FerraColumns,
  FerraEditorialRow,
  FerraInstrument,
  FerraNotes,
  FerraProductLead,
  FerraProductPage,
  FerraReleaseFacts,
} from "ferramenta-family/ferra";
import { family, WORKSHOP } from "ferramenta-family/registry";

import { AuditExample } from "../components/audit-example";
import {
  CapabilitiesSection,
  CatalogSection,
  ClosingSection,
  GettingStartedSection,
  ReviewSection,
} from "../components/home-sections";
import { version } from "../version";

// oxlint-disable-next-line react/only-export-components -- Ardo reads route metadata.
export const meta: ArdoMetaFunction = () => [
  { title: "Ferrocat — Keep your translations ready to ship." },
  {
    name: "description",
    content:
      "Rust-native translation catalogs for your repository. Parse PO and FCL, review human corrections, audit releases and compile ICU messages for your application.",
  },
];
// oxlint-disable-next-line react/only-export-components -- Ardo reads the route layout.
export const handle = { layout: "bare" };

const member = family.find((entry) => entry.name === "ferrocat");
if (!member) throw new Error("Ferrocat is missing from the family registry.");
const tool = { ...member, version };

function Opening() {
  return (
    <FerraProductLead
      id="overview"
      title="Keep your translations ready to ship."
      introduction="A Rust-native translation catalog engine for teams who keep translations in their repository. Read PO and ICU messages, update catalogs, review changes and catch release problems before they reach your users."
      art={<FerraInstrument product="ferrocat" />}
      byline={
        <>
          An open-source project by <a href={WORKSHOP.openSource}>{WORKSHOP.company}</a>.
        </>
      }
    >
      <div className="ferra-actions">
        <FerraAction href="/guide/getting-started">Start with Rust</FerraAction>
        <FerraAction variant="secondary" href="#audit">
          See a catalog audit
        </FerraAction>
      </div>
    </FerraProductLead>
  );
}

function Overview() {
  return (
    <FerraColumns>
      <FerraArticle
        title="Keep the catalog. Improve the workflow."
        icon={<Mark name="document" size={28} />}
        meta={<FerraReleaseFacts tool={tool} />}
      >
        <p>
          Bring existing PO files or choose ICU-native FCL for machine-managed catalogs. One Rust
          core handles parsing, updates, review and runtime compilation.
        </p>
        <Link to="/guide/catalog-modes">Compare the storage choices</Link>
      </FerraArticle>
      <FerraArticle
        title="Turn translation gaps into useful diagnostics."
        icon={<Mark name="check" size={28} />}
        meta={<Link to="/quality/conformance">How compatibility is checked</Link>}
      >
        <p>
          Find missing translations, stale targets and ICU mistakes before release.
          Source-attributed conformance fixtures keep the behavior tied to real catalog tools and
          message formats.
        </p>
        <a href="#audit">Inspect a real CLI result</a>
      </FerraArticle>
    </FerraColumns>
  );
}

function PerformanceSection() {
  return (
    <FerraArticle
      title="Catalog work should keep up with your build."
      meta={<Link to="/performance/results">Published comparisons and measurement conditions</Link>}
    >
      <p>
        The parser borrows from the input where it can, and high-level catalog operations build on
        the same Rust core. Recorded comparisons cover parsing and updates across Rust, Node.js,
        PHP, Python and GNU Gettext.
      </p>
      <p>
        The detailed results keep task differences, source revisions and measurement dates visible.
        Use the fixture suite to measure the workload that matters to your application.
      </p>
    </FerraArticle>
  );
}

export default function HomePage() {
  return (
    <RegistryFacts>
      <FerraProductPage>
        <Opening />
        <Overview />
        <AuditExample />
        <FerraEditorialRow
          notes={
            <FerraNotes
              label="Inside the catalog format"
              notes={tool.editorial?.notes.slice(1) ?? []}
            />
          }
        >
          <CatalogSection />
        </FerraEditorialRow>
        <FerraEditorialRow
          notes={
            <FerraNotes
              label="Why the correction matters"
              notes={tool.editorial?.notes.slice(0, 1) ?? []}
            />
          }
        >
          <ReviewSection />
        </FerraEditorialRow>
        <CapabilitiesSection />
        <PerformanceSection />
        <GettingStartedSection />
        <ClosingSection />
      </FerraProductPage>
    </RegistryFacts>
  );
}
