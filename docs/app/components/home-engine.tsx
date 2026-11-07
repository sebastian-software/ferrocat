import { ArdoLink as Link } from "ardo/runtime";
import {
  MonoCredit,
  MonoLeadIn,
  MonoPassage,
  MonoProofs,
  MonoSection,
  MonoTerms,
  MonoWide,
} from "ferramenta-family/monografie";

import { ReviewSpecimen, RuntimeSpecimen } from "./catalog-specimens";
import { ferrocat, lowerFirst, palamedes, registryNote, samples, spelled } from "./home-data";
import { SpeedComparison } from "./speed-comparison";

/* The homepage's engine chapters: review, runtime, speed, conformance and fit. */

const { conformance, speed } = samples;
const capitalized = (text: string) => `${text.charAt(0).toUpperCase()}${text.slice(1)}`;
const measuredOn = new Date(`${speed.measured.date}T00:00:00Z`).toLocaleDateString("en-US", {
  year: "numeric",
  month: "long",
  day: "numeric",
  timeZone: "UTC",
});

export function Review() {
  const note = registryNote("The correction matters");
  return (
    <MonoSection
      id="review"
      title="A human correction leaves a trace."
      standfirst="Machine translation can write the first draft. When a person improves it, the catalog knows."
    >
      <MonoPassage note={{ term: note.title, children: note.body }}>
        <p>
          <MonoLeadIn>Many teams let a model</MonoLeadIn> translate new strings and ask a person to
          review the result. The trouble starts on the next run: which values did the machine write,
          and which did a reviewer fix? Overwrite the wrong one, and a careful correction is gone.
        </p>
      </MonoPassage>
      <p>
        When a tool writes a translation, Ferrocat can store a fingerprint of exactly that value
        beside it, with the model and its confidence. As long as the text and the fingerprint agree,
        the value is still machine output. Once a person edits it, they no longer agree, and every
        later review or translation run can see that a human stepped in. Ferrocat reports it; your
        policy decides what happens next.
      </p>
      <ReviewSpecimen />
      <MonoCredit>
        <Link to="/reference/api-overview#machine-managed-value-metadata">
          Fingerprints and machine-translation metadata
        </Link>
      </MonoCredit>
    </MonoSection>
  );
}

export function Runtime() {
  return (
    <MonoSection
      id="runtime"
      title="Compile what the application loads."
      standfirst="One file per language, keyed for fast lookup, with every fallback written down."
    >
      <MonoPassage
        note={{
          term: "Stable key",
          children:
            "A short ID derived from the sentence and its context. It changes only when they do.",
        }}
      >
        <p>
          <MonoLeadIn>An application does not need</MonoLeadIn> comments, origins or review marks
          while it runs. It needs the right text for a key, quickly. Ferrocat compiles the catalogs
          for one language into exactly that: a map from stable keys to messages, ready to ship as
          JSON beside your code.
        </p>
      </MonoPassage>
      <p>
        Gaps do not disappear in the process. A message without a translation falls back to the next
        language you name, and the artifact lists it under <code>missing</code>, so the build can
        warn or stop.
      </p>
      <RuntimeSpecimen />
      <MonoCredit>
        <Link to="/reference/api-overview#compile-catalog-artifact">
          Runtime artifacts and fallback chains
        </Link>
      </MonoCredit>
    </MonoSection>
  );
}

/** How the speed figures were measured, from the committed reports. */
function Conditions() {
  return (
    <>
      <h3>How this was measured</h3>
      <p>
        Recorded on {measuredOn} on an {speed.measured.machine}, with Ferrocat{" "}
        {speed.measured.ferrocat}: the latest run in the repository. The current release has not
        been measured with this profile yet.
      </p>
      <p>
        One German catalog of {speed.measured.messages.toLocaleString("en-US")} messages. Each
        figure compares median throughput from the same report.
      </p>
      <p>
        GNU msgmerge runs as its own process; the time it takes to start is subtracted. The tools do
        different amounts of checking and preserving, so read the task notes before you compare.
      </p>
    </>
  );
}

export function Speed() {
  return (
    <MonoSection
      id="speed"
      title="Catalog work at the speed of your build."
      standfirst="Reading a catalog and adding new strings to it, Ferrocat finished ahead of GNU msgmerge and the gettext libraries we compared for Node.js, PHP and Python."
    >
      <MonoPassage
        note={{
          figure: speed.measured.messages.toLocaleString("en-US"),
          children: "messages in the German catalog every tool worked on",
        }}
      >
        <p>
          <MonoLeadIn>Catalog work runs</MonoLeadIn> more often than people expect: on every
          extraction, in every pull request that touches a string, before every release, once per
          language. On a large product, that is many thousands of messages each time. When the
          catalog step takes seconds, developers start skipping it.
        </p>
      </MonoPassage>
      <p>
        Ferrocat scans the bytes of a catalog directly, and its update moves messages from the old
        catalog into the new one instead of copying them. Switch the chart between the two everyday
        jobs.
      </p>
      <MonoWide>
        <SpeedComparison
          after={
            <p>
              The published comparisons also cover pofile, pofile-ts and Babel, and a full update
              with ICU checks. <Link to="/performance/results">Every task and its conditions</Link>
            </p>
          }
          conditions={<Conditions />}
        />
      </MonoWide>
    </MonoSection>
  );
}

const SUITES: Record<string, string> = {
  "po-polib": "polib, the Python PO library",
  "po-pofile": "pofile, the JavaScript PO library",
  "po-babel": "Babel, Python’s i18n toolkit",
  "icu-official": "ICU, the official MessageFormat tests",
  "icu-ecosystem": "FormatJS message syntax",
};

export function Conformance() {
  const order = ["po-polib", "po-pofile", "po-babel", "icu-official", "icu-ecosystem"];
  const suites = [...conformance.suites].sort((a, b) => order.indexOf(a.id) - order.indexOf(b.id));
  return (
    <MonoSection
      id="conformance"
      title="Held to the tests of the tools teams know."
      standfirst="PO has quirks that only show up in real catalogs. Ferrocat runs cases taken from the libraries that met them first."
    >
      <MonoPassage
        note={{
          figure: conformance.assertions,
          children: `checks in ${spelled(conformance.cases)} cases, none of them failing`,
        }}
      >
        <p>
          <MonoLeadIn>A catalog engine</MonoLeadIn> is only as good as its odd cases: escaped
          quotes, a byte-order mark, obsolete entries, contexts, values spread over several lines.
          The libraries below have collected such cases for years. Ferrocat adapts their tests,
          keeps the source of each one, and runs them with its own test suite.
        </p>
      </MonoPassage>
      <p>
        Most cases must parse and match the expected structure.{" "}
        {capitalized(spelled(conformance.expectedReject))} must be rejected, because the upstream
        test expects an error there.
      </p>
      <MonoProofs
        items={suites.map((suite) => ({
          label: SUITES[suite.id] ?? suite.id,
          value: `${String(suite.assertions)} / ${String(suite.assertions)}`,
        }))}
      />
      <MonoCredit>
        Counted by the repository’s conformance harness.{" "}
        <Link to="/quality/conformance">Sources, licenses and scope</Link>
      </MonoCredit>
    </MonoSection>
  );
}

export function Fit() {
  return (
    <MonoSection id="fit" title="A focused engine. Your application around it.">
      <p>
        Ferrocat is {lowerFirst(ferrocat.what)}. It {lowerFirst(ferrocat.does)} It leaves the rest
        of internationalization to the tools around it, and you need none of them to use it.
      </p>
      <MonoWide>
        <MonoTerms
          prose
          items={[
            {
              term: "Rust applications and tools",
              description: (
                <>
                  Add the library with <code>cargo add ferrocat</code> and call it directly.
                </>
              ),
            },
            {
              term: "Release checks in CI",
              description: (
                <>
                  Install the command line with <code>cargo install ferrocat-cli</code> and audit on
                  every build.
                </>
              ),
            },
            {
              term: "TypeScript applications",
              description: (
                <>
                  <a href={palamedes.docs}>Palamedes</a>, {lowerFirst(palamedes.what)}, runs on
                  Ferrocat. There is no npm package of Ferrocat itself.
                </>
              ),
            },
            {
              term: "Not included",
              description:
                "Finding strings in your source code, translating them, or running a translation platform. Those stay with your tools.",
            },
          ]}
        />
      </MonoWide>
    </MonoSection>
  );
}
