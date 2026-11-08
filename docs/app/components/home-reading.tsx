import { ArdoLink as Link } from "ardo/runtime";
import { MonoCredit, MonoPassage, MonoSection } from "ferramenta-family/monografie";

import { AuditSpecimen } from "./audit-specimen";
import { IcuSpecimen, StorageSpecimen, UpdateSpecimen } from "./catalog-specimens";
import { registryNote } from "./home-data";

/* The homepage's reading chapters: what a catalog is, its formats, ICU checks, the audit. */

export function Explainer() {
  return (
    <MonoSection id="overview" title="Every sentence your app says, in one catalog.">
      <MonoPassage
        note={{
          term: "Translation catalog",
          children:
            "A file that pairs every sentence of an application with its translation, one file per language.",
        }}
      >
        <p>
          An application speaks in hundreds of small sentences: a button label, a greeting, a line
          that counts the items in a cart. To show them in German or Japanese, a team collects every
          sentence from the code into a translation catalog. Translators fill in the other side, and
          the application looks each sentence up while it runs. The catalog lives next to the code,
          in the same repository, and changes with it.
        </p>
      </MonoPassage>
      <MonoPassage
        note={{
          side: "right",
          term: "PO file",
          children:
            "The catalog format of GNU gettext. Most translation tools and services still read it.",
        }}
      >
        <p>
          The most common catalog is the PO file of GNU gettext. Each entry holds the sentence in
          the source language, its translation, the place in the code it came from and a note for
          the translator. Ferrocat reads and writes PO, so the tools your translators use today keep
          working.
        </p>
      </MonoPassage>
      <p>
        The hard part is not reading the file. It is keeping it right while the code moves on.
        Developers add sentences, reword others and delete a few. Every change has to reach every
        language without losing a translation someone already paid for. Ferrocat compares what the
        code says now with what the catalog holds, adds what is new, keeps what still matches, and
        sets aside what the code no longer uses, with the date it happened.
      </p>
      <UpdateSpecimen />
    </MonoSection>
  );
}

export function Catalogs() {
  const note = registryNote("One translation, one line");
  return (
    <MonoSection
      id="catalogs"
      title="Translator-friendly files. Git-friendly lines."
      standfirst="Keep PO where translation tools expect it. Choose FCL where machines write the catalog and Git has to merge it."
    >
      <MonoPassage note={{ term: "FCL", children: note.body }}>
        <p>
          A PO entry spans several lines: comments, a context, the source, the translation. When two
          branches both touch a catalog, Git compares those lines, and edits that have nothing to do
          with each other can still meet in one conflict. Ferrocat Catalog Lines, FCL for short,
          writes each message on exactly one line and sorts the lines in a fixed order. Unrelated
          edits land on different lines, and an entry can never be torn apart by a merge.
        </p>
      </MonoPassage>
      <p>
        You choose the format per catalog, and say so explicitly. Classic PO keeps gettext’s own
        plural rules. ICU PO and FCL carry ICU messages. Converting between ICU PO and FCL keeps
        every message, with its origin and its notes. Switch the sheet below to see the same German
        catalog both ways.
      </p>
      <StorageSpecimen />
      <MonoCredit>
        <Link to="/guide/catalog-modes">How to choose a catalog mode</Link>
      </MonoCredit>
    </MonoSection>
  );
}

export function Messages() {
  return (
    <MonoSection
      id="messages"
      title="Plurals and placeholders, checked."
      standfirst="A translation can read perfectly and still break the screen. Ferrocat compares its structure with the source before your users see it."
    >
      <MonoPassage
        note={{
          term: "ICU MessageFormat",
          children:
            "A syntax for messages with placeholders, plurals and choices, used by FormatJS, Lingui and many other i18n libraries.",
        }}
      >
        <p>
          “1 items in your cart” is the classic mistake. ICU messages avoid it: a message names its
          placeholders in braces and lists one wording per plural form, and the application picks
          the right one at run time. The source below says “1 item” and “3 items” correctly, and the
          German translation has to keep that structure.
        </p>
      </MonoPassage>
      <MonoPassage
        note={{
          side: "right",
          term: "Placeholder",
          children:
            "A named gap in a message, like {count}, that the application fills in. Its name must stay in the source language.",
        }}
      >
        <p>
          Translators are careful people, and placeholders still get translated, dropped or
          flattened. The runtime then shows a raw brace, an empty gap or the wrong plural. Ferrocat
          parses both messages and compares them: every placeholder the source needs, every plural
          and choice, every formatting style and markup tag.
        </p>
      </MonoPassage>
      <IcuSpecimen />
    </MonoSection>
  );
}

export function Audit() {
  return (
    <MonoSection
      id="audit"
      title="Find the missing words before your users do."
      standfirst="Point the command-line tool at your catalogs. It reports every gap, and its exit status fails the build."
    >
      <MonoPassage
        note={{
          term: "Exit status",
          children:
            "The number a command returns when it ends. A CI pipeline treats anything but 0 as a failed step.",
        }}
      >
        <p>
          A release audit compares every target language with the source catalog: translations that
          are missing or empty, entries still marked for review, ICU messages that do not parse or
          do not match their source. Errors and warnings come with the language, the message and a
          stable code, in plain text for people and as JSON for your tooling.
        </p>
      </MonoPassage>
      <p>
        Put it in the pipeline that builds your release. A missing translation then stops the merge,
        not a customer.
      </p>
      <AuditSpecimen />
      <MonoCredit>
        <Link to="/reference/api-overview#audit-catalogs">Audit checks and report formats</Link>
      </MonoCredit>
    </MonoSection>
  );
}
