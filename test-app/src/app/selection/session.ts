import type {
  SelectedValue,
  SelectionStats,
  SelectionTarget,
} from "../../engines/selection";
import { createRustSelection } from "../../engines/rust-selection";
import { selectionScenario } from "../../scenarios/selection";
import { compareSelectionScenario } from "../../traces/selection";

export type SelectionView = {
  values: Record<SelectionTarget, SelectedValue>;
  stats: SelectionStats;
  comparison?: ReturnType<typeof compareSelectionScenario>;
  error?: string;
};

export class SelectionSession {
  private readonly engine = createRustSelection();
  private readonly listeners = new Set<() => void>();
  private view: SelectionView = this.snapshot();

  constructor() {
    this.engine.subscribe("source", () => this.update(this.snapshot()));
  }

  getSnapshot = () => this.view;

  subscribe = (callback: () => void) => {
    this.listeners.add(callback);
    return () => this.listeners.delete(callback);
  };

  increment = () => {
    try {
      this.engine.send({ type: "inc", by: 1 });
      this.update({ error: undefined });
    } catch (error) {
      this.update({
        error: error instanceof Error ? error.message : String(error),
      });
    }
  };

  compare = () => {
    try {
      this.update({
        comparison: compareSelectionScenario(selectionScenario),
        error: undefined,
      });
    } catch (error) {
      this.update({
        error: error instanceof Error ? error.message : String(error),
      });
    }
  };

  dispose = () => {
    this.engine.dispose();
    this.listeners.clear();
  };

  private snapshot(): Pick<SelectionView, "values" | "stats"> {
    return {
      values: {
        source: this.engine.read("source"),
        mapped: this.engine.read("mapped"),
        chain: this.engine.read("chain"),
      },
      stats: this.engine.stats(),
    };
  }

  private update(change: Partial<SelectionView>) {
    this.view = { ...this.view, ...change };
    for (const callback of this.listeners) callback();
  }
}
