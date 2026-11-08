import type { ArdoMetaFunction } from "ardo/runtime";

import { Fragment } from "ardo/react";
import { ArdoLink as Link } from "ardo/runtime";
import {
  MonoAction,
  MonoClosing,
  MonoColophon,
  MonoEngraving,
  MonoOpening,
} from "ferramenta-family/monografie";
import { WORKSHOP } from "ferramenta-family/registry";

import { ferrocat, lowerFirst, samples } from "../components/home-data";
import { Conformance, Fit, Review, Runtime, Speed } from "../components/home-engine";
import { Audit, Catalogs, Explainer, Messages } from "../components/home-reading";
import { rustVersion, version } from "../data/site.json";

// oxlint-disable-next-line react/only-export-components -- Ardo reads route metadata.
export const meta: ArdoMetaFunction = () => [
  { title: "Ferrocat — Translation catalogs, ready to ship" },
  {
    name: "description",
    content:
      "A Rust engine for translation catalogs: update PO, ICU and FCL catalogs as the code changes, check every translation, keep human corrections visible and compile what your application loads.",
  },
];
// oxlint-disable-next-line react/only-export-components -- Ardo reads the route layout.
export const handle = { layout: "bare" };

const releaseLine = version.split(".").slice(0, 2).join(".");
const { conformance } = samples;

function Actions() {
  return (
    <>
      <MonoAction href="/guide/getting-started" primary detail="cargo add ferrocat">
        Start with Rust
      </MonoAction>
      <MonoAction href="/reference/api-overview#audit-catalogs" detail="cargo install ferrocat-cli">
        Audit your catalogs
      </MonoAction>
    </>
  );
}

function Opening() {
  return (
    <MonoOpening
      product="ferrocat"
      instrumentLabel="Ferrocat's instrument: two steel card indexes full of tabbed leaves on either side of a machined rail, and between them a single rust-brown leaf standing in a slotted comb above an engraved scale."
      kicker={`Ferrocat ${releaseLine}, ${lowerFirst(ferrocat.what)} for Rust`}
      title="Your translations, ready to ship."
      subtitle="Catalogs that Git can merge and people can trust."
      standfirst={`Every sentence an application shows lives in a translation catalog. Ferrocat keeps those catalogs right as the code changes: it updates them, checks what translators wrote, keeps human corrections visible and compiles what your application loads. It reads Gettext PO, ICU messages and FCL, a format with one line per message. ${ferrocat.audience}`}
      actions={<Actions />}
      facts={[
        {
          value: (
            <>
              <em>{conformance.assertions}</em> of {conformance.assertions}
            </>
          ),
          label: "conformance checks from polib, pofile, Babel, ICU and FormatJS",
        },
        { value: "3 modes", label: "Gettext PO, ICU PO and FCL, chosen explicitly" },
        {
          value: (
            <>
              <em>1</em> line
            </>
          ),
          label: "per message in FCL, so unrelated edits merge cleanly",
        },
        { value: `Rust ${rustVersion}+`, label: "MIT or Apache-2.0, library and CLI" },
      ]}
    />
  );
}

// Keys are written out: minified builds do not keep function names.
const chapters = [
  { key: "explainer", Chapter: Explainer },
  { key: "catalogs", Chapter: Catalogs },
  { key: "messages", Chapter: Messages },
  { key: "audit", Chapter: Audit },
  { key: "review", Chapter: Review },
  { key: "runtime", Chapter: Runtime },
  { key: "speed", Chapter: Speed },
  { key: "conformance", Chapter: Conformance },
  { key: "fit", Chapter: Fit },
];

/** Where Ferrocat comes from, and who makes it. */
function Origin() {
  return (
    <section aria-label="Origin">
      <div className="mono-axis">
        <MonoColophon>
          {ferrocat.proof} Ferrocat continues pofile-ts, its TypeScript predecessor, which stays
          available as an archived reference. It is one of the Ferramenta engines, made by{" "}
          {WORKSHOP.name} in {WORKSHOP.place}.
        </MonoColophon>
      </div>
    </section>
  );
}

export default function HomePage() {
  return (
    <>
      <Opening />
      {chapters.map(({ key, Chapter }, index) => (
        <Fragment key={key}>
          <MonoEngraving seed={index + 1} />
          <Chapter />
        </Fragment>
      ))}
      <MonoEngraving seed={chapters.length + 1} />
      <Origin />
      <div id="start">
        <MonoClosing
          title="Put your catalogs in order."
          standfirst="Start with the Rust library, or add the audit to your release pipeline first. Both read the catalogs you already have."
          actions={<Actions />}
          links={
            <>
              <a href="https://github.com/sebastian-software/ferrocat">GitHub repository</a>
              <Link to="/guide/getting-started">Documentation</Link>
              <a href="https://docs.rs/ferrocat">API reference</a>
              <a href={WORKSHOP.consulting}>Integration and support</a>
            </>
          }
        />
      </div>
    </>
  );
}
