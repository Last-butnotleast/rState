import type { Event, Scenario, ScenarioName } from "../engines/contract";
import { actions } from "../scenarios/catalog";
import type { LabView } from "./session";

type Props = {
  scenario: Scenario;
  scenarios: Scenario[];
  view?: LabView;
  error?: string;
  onScenario(name: ScenarioName): void;
  onSend(event: Event): void;
  onCompare(): void;
};

export function LabDisplay({
  scenario,
  scenarios,
  view,
  error,
  onScenario,
  onSend,
  onCompare,
}: Props) {
  const comparison = view?.comparison;
  return (
    <main>
      <header className="masthead">
        <a className="wordmark" href="/">
          rstate<span> / core lab</span>
        </a>
        <span className="runtime">
          <i />
          Rust · WebAssembly
        </span>
      </header>
      <section className="intro">
        <p className="eyebrow">SMALL CORE. PRECISE BEHAVIOR.</p>
        <h1>
          State you can
          <br />
          <em>reason about.</em>
        </h1>
        <p className="description">
          Exercise the Rust core. Inspect every transition.
          <br />
          Compare the result with the pinned XState Store reference.
        </p>
      </section>
      <section className="workspace">
        <div className="toolbar">
          <nav aria-label="Scenario">
            {scenarios.map((option) => (
              <button
                key={option.scenario}
                className={
                  option.scenario === scenario.scenario ? "tab active" : "tab"
                }
                aria-pressed={option.scenario === scenario.scenario}
                onClick={() => onScenario(option.scenario)}
              >
                {option.scenario === "composition"
                  ? "Cart composition"
                  : option.scenario === "ordering"
                    ? "Effect ordering"
                    : "Counter"}
              </button>
            ))}
          </nav>
          <span className="reference">
            reference <code>9b3a88f</code>
          </span>
        </div>
        <div className="panels">
          <section className="control-panel" aria-label="Rust store">
            <div className="panel-heading">
              <span className="number">01</span>
              <h2>Live Rust store</h2>
            </div>
            <p className="scenario-name">{scenario.name}</p>
            <div className="metric" data-testid="metric">
              {scenario.scenario === "composition"
                ? (view?.snapshot.context.total ?? "—")
                : (view?.snapshot.context.count ?? "—")}
              <span>
                {scenario.scenario === "composition"
                  ? "cart total"
                  : "current count"}
              </span>
            </div>
            <div className="actions">
              {actions[scenario.scenario].map((action) => (
                <button
                  className="action"
                  key={action.label}
                  disabled={!view}
                  onClick={() => onSend(action.event)}
                >
                  {action.label}
                  <span aria-hidden="true">↗</span>
                </button>
              ))}
            </div>
            <div className="snapshot-label">
              CURRENT SNAPSHOT <span>#{view?.snapshot.snapshotId ?? "—"}</span>
            </div>
            <pre className="snapshot" data-testid="snapshot">
              {JSON.stringify(view?.snapshot.context ?? {}, null, 2)}
            </pre>
          </section>
          <section className="trace-panel" aria-label="Ordered trace">
            <div className="panel-heading">
              <span className="number">02</span>
              <h2>Ordered trace</h2>
              <span className="count">{view?.trace.length ?? 0} entries</span>
            </div>
            <div className="trace-list" data-testid="trace">
              {!view?.trace.length && (
                <div className="empty-trace">
                  <span>↳</span>
                  <p>Send an event to follow its path.</p>
                  <small>Dispatch, notifications, effects — in order.</small>
                </div>
              )}
              {view?.trace.map((entry, index) => (
                <div className="trace-entry" key={index}>
                  <span className="trace-index">
                    {String(index + 1).padStart(2, "0")}
                  </span>
                  <span className={`trace-kind ${entry.kind}`}>
                    {String(entry.kind)}
                  </span>
                  <code>
                    {JSON.stringify(
                      Object.fromEntries(
                        Object.entries(entry).filter(([key]) => key !== "kind"),
                      ),
                    )}
                  </code>
                </div>
              ))}
            </div>
          </section>
        </div>
        <section className="comparison" aria-label="Reference comparison">
          <div>
            <p className="eyebrow">BEHAVIORAL PARITY</p>
            <h2>
              {comparison
                ? comparison.equal
                  ? "Every trace matches."
                  : `${comparison.differences.length} trace differences.`
                : "Same scenario. Two implementations."}
            </h2>
            <p>
              Compare values, identity, callbacks, emissions and effect order.
            </p>
          </div>
          <div className="comparison-action">
            <span
              role="status"
              className={
                comparison
                  ? comparison.equal
                    ? "status pass"
                    : "status fail"
                  : "status"
              }
            >
              {comparison
                ? comparison.equal
                  ? `✓ ${comparison.rust.length} matching entries`
                  : "Review differences below"
                : "Ready to compare"}
            </span>
            <button
              className="compare-button"
              onClick={onCompare}
              disabled={!view}
            >
              Run comparison <span>→</span>
            </button>
          </div>
        </section>
        {(error ?? view?.error) && (
          <p role="alert" className="error">
            {error ?? view?.error}
          </p>
        )}
        {comparison && !comparison.equal && (
          <pre className="differences">
            {JSON.stringify(
              comparison.differences.map((index) => ({
                index,
                rust: comparison.rust[index],
                reference: comparison.reference[index],
              })),
              null,
              2,
            )}
          </pre>
        )}
      </section>
      <footer>
        <span>Native Rust core. Real browser execution.</span>
        <span>Synchronous stores · core v0.1</span>
      </footer>
    </main>
  );
}
