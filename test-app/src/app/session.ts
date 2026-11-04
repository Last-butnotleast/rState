import type {
  Event,
  Scenario,
  Snapshot,
  TraceEntry,
} from "../engines/contract";
import { createRust } from "../engines/rust";
import { compareScenario } from "../traces/compare";

export type LabView = {
  snapshot: Snapshot;
  trace: TraceEntry[];
  comparison?: ReturnType<typeof compareScenario>;
  error?: string;
};

export class LabSession {
  private readonly listeners = new Set<() => void>();
  private readonly engine;
  private view: LabView;

  constructor(readonly scenario: Scenario) {
    this.engine = createRust(scenario.scenario, (entry) => this.append(entry));
    this.view = { snapshot: this.engine.snapshot(), trace: [] };
    this.engine.subscribe((snapshot) => {
      this.append({ kind: "observe", snapshot });
      this.update({ snapshot });
    });
    this.engine.onAny((message) => this.append({ kind: "emit", message }));
  }

  getSnapshot = () => this.view;

  subscribe = (callback: () => void) => {
    this.listeners.add(callback);
    return () => {
      this.listeners.delete(callback);
    };
  };

  send = (event: Event) => {
    this.append({ kind: "enter", op: "send", event });
    try {
      this.engine.send(event);
      this.append({ kind: "return", op: "send" });
      this.update({ snapshot: this.engine.snapshot(), error: undefined });
    } catch (error) {
      this.update({
        error: error instanceof Error ? error.message : String(error),
      });
    }
  };

  compare = () => {
    try {
      this.update({
        comparison: compareScenario(this.scenario),
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

  private append(entry: TraceEntry) {
    this.update({ trace: [...this.view.trace, structuredClone(entry)] });
  }

  private update(change: Partial<LabView>) {
    this.view = { ...this.view, ...change };
    this.listeners.forEach((callback) => callback());
  }
}
