import type { SelectionTarget } from "../../engines/selection";
import type { SelectionView } from "./session";

type Props = {
  view?: SelectionView;
  error?: string;
  onIncrement(): void;
  onCompare(): void;
};

const targets: SelectionTarget[] = ["source", "mapped", "chain"];

export function SelectionDisplay({
  view,
  error,
  onIncrement,
  onCompare,
}: Props) {
  return (
    <section
      className="workspace factory-workspace"
      aria-label="Selection composition"
    >
      <div className="toolbar">
        <h2>One source. Explicit derived selections.</h2>
        <button className="action" disabled={!view} onClick={onIncrement}>
          Increment source
        </button>
      </div>
      <div className="selection-values">
        {targets.map((target) => (
          <article className="control-panel" key={target}>
            <h2>{target}</h2>
            <div
              className="selection-metric"
              data-testid={`selection-${target}`}
            >
              {view?.values[target].value ?? "—"}
            </div>
            <p className="scenario-name">
              Identity #{view?.values[target].id ?? "—"} ·{" "}
              {view?.stats[target] ?? 0} calculations
            </p>
          </article>
        ))}
      </div>
      <div className="comparison">
        <div>
          <h2>Count → floor(count / 2) → label</h2>
          <p aria-live="polite" data-testid="selection-comparison">
            {view?.comparison
              ? view.comparison.equal
                ? `${view.comparison.rust.length} selection trace entries match.`
                : `${view.comparison.differences.length} selection trace differences.`
              : "Unchanged buckets retain identity and skip the final calculation."}
          </p>
        </div>
        <button className="compare-button" disabled={!view} onClick={onCompare}>
          Compare selections
        </button>
      </div>
      {(error ?? view?.error) && (
        <p className="error" role="alert">
          {error ?? view?.error}
        </p>
      )}
      {view?.comparison && !view.comparison.equal && (
        <pre className="differences">
          {JSON.stringify(view.comparison, null, 2)}
        </pre>
      )}
    </section>
  );
}
