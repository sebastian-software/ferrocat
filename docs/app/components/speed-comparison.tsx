import { type ReactNode, useState } from "ardo/react";
import {
  MonoBarChart,
  type MonoBarRow,
  MonoSegmented,
  MonoTableView,
} from "ferramenta-family/monografie";

import { samples } from "./home-data";

/* Time for the same work, relative to Ferrocat, from the committed benchmark
   reports via app/data/home-samples.json. */

const { speed } = samples;
type TaskId = "merge" | "parse";

/** The task, how the chart scales it, and the language each tool is written in. */
const TASKS: Record<TaskId, { label: string; phrase: string; scale: number; ticks: number[] }> = {
  parse: { label: "Read a catalog", phrase: "reading a catalog", scale: 25, ticks: [1, 10, 20] },
  merge: {
    label: "Add new strings",
    phrase: "adding new strings to a catalog",
    scale: 40,
    ticks: [1, 10, 20, 30, 40],
  },
};
const LANGUAGES: Record<string, string> = {
  "gettext-parser": "JavaScript, Node.js",
  "php-gettext": "PHP",
  polib: "Python",
  msgmerge: "C, GNU gettext",
};

function task(id: TaskId) {
  const found = speed.tasks.find((entry) => entry.id === id);
  if (!found) throw new Error(`The speed data has no ${id} task.`);
  return found;
}

function rowsFor(id: TaskId): MonoBarRow[] {
  return [
    { id: "ferrocat", label: "Ferrocat", sub: "Rust", value: 1, own: true },
    ...task(id).rows.map((row) => ({
      id: row.id,
      label: row.label,
      sub: LANGUAGES[row.id] ?? "",
      value: row.ratio,
    })),
  ];
}

function describeFor(id: TaskId) {
  return function describe(row: MonoBarRow, cut: boolean) {
    if (row.own === true) {
      return (
        <>
          <b>Ferrocat</b> is the baseline: 1× for {TASKS[id].phrase}.
        </>
      );
    }
    return (
      <>
        <b>{row.label}</b> needed {row.value.toFixed(2)}× as long as Ferrocat for {TASKS[id].phrase}
        {cut ? "; its bar runs past the end of the scale." : "."}
      </>
    );
  };
}

/** Every figure of the chart as a table, with the tool versions measured. */
function Figures() {
  return (
    <MonoTableView>
      <table className="mono-table">
        <thead>
          <tr>
            <th scope="col">Task</th>
            <th scope="col">Tool</th>
            <th scope="col">Version measured</th>
            <th scope="col" className="mono-num">
              Time
            </th>
          </tr>
        </thead>
        <tbody>
          {speed.tasks.flatMap((entry) =>
            entry.rows.map((row) => (
              <tr key={`${entry.id}-${row.id}`}>
                <td>{TASKS[entry.id === "parse" ? "parse" : "merge"].label}</td>
                <td>{row.label}</td>
                <td>{row.tool}</td>
                <td className="mono-num">{row.ratio.toFixed(2)}×</td>
              </tr>
            )),
          )}
        </tbody>
      </table>
    </MonoTableView>
  );
}

export type SpeedComparisonProps = {
  /** Text and links under the scale. */
  after: ReactNode;
  /** The measuring conditions beside the chart. */
  conditions: ReactNode;
};

export function SpeedComparison({ after, conditions }: SpeedComparisonProps) {
  const [id, setId] = useState<TaskId>("merge");
  const { phrase, scale, ticks } = TASKS[id];
  return (
    <MonoBarChart
      label={`Time for ${phrase}, relative to Ferrocat`}
      title="Time for the same work"
      description="Ferrocat = 1×, the line through every row. A bar past it took longer."
      toggles={
        <MonoSegmented
          label="Task"
          value={id}
          options={[
            { value: "merge", label: TASKS.merge.label },
            { value: "parse", label: TASKS.parse.label },
          ]}
          onChange={(value) => {
            setId(value === "parse" ? "parse" : "merge");
          }}
        />
      }
      rows={rowsFor(id)}
      scale={scale}
      ticks={ticks}
      describe={describeFor(id)}
      after={
        <>
          {after}
          <Figures />
        </>
      }
      conditions={conditions}
    />
  );
}
