import type { Event } from "../../engines/contract";
import type { FactoryView } from "./session";

type Props = {
  view?: FactoryView;
  error?: string;
  onSend(name: string, event: Event): void;
  onCompare(): void;
};

export function FactoryDisplay({ view, error, onSend, onCompare }: Props) {
  return (
    <section
      className="workspace factory-workspace"
      aria-label="Reusable store factory"
    >
      <div className="toolbar">
        <h2>One factory. Independent stores.</h2>
        <span className="reference">input-initialized instances</span>
      </div>
      <div className="factory-instances">
        {view?.instances.map(({ name, input, snapshot }) => (
          <article
            className="control-panel"
            key={name}
            aria-label={`${name} instance`}
          >
            <h2>{name}</h2>
            <p className="scenario-name">Initial count {input.count}</p>
            <div className="metric" data-testid={`factory-${name}`}>
              {snapshot.context.count}
              <span>current count</span>
            </div>
            <button
              className="action"
              onClick={() => onSend(name, { type: "inc", by: 1 })}
            >
              Increment {name}
            </button>
          </article>
        ))}
      </div>
      <div className="comparison">
        <div>
          <h2>Shared definition, isolated state and observations.</h2>
          <p aria-live="polite" data-testid="factory-comparison">
            {view?.comparison
              ? view.comparison.equal
                ? `${view.comparison.rust.length} factory trace entries match.`
                : `${view.comparison.differences.length} factory trace differences.`
              : "Compare both input-created stores with the pinned reference."}
          </p>
        </div>
        <button className="compare-button" disabled={!view} onClick={onCompare}>
          Compare factory
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
