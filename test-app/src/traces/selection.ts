import type { TraceEntry } from "../engines/contract";
import { createReferenceSelection } from "../engines/reference-selection";
import { createRustSelection } from "../engines/rust-selection";
import type {
  SelectionEngine,
  SelectionOperation,
  SelectionScenario,
} from "../engines/selection";
import { compareTraces } from "./compare";

export function compareSelectionScenario(scenario: SelectionScenario) {
  return compareTraces(
    runSelectionScenario(scenario, createRustSelection),
    runSelectionScenario(scenario, createReferenceSelection),
  );
}

export function runSelectionScenario(
  scenario: SelectionScenario,
  create: () => SelectionEngine,
): TraceEntry[] {
  const engine = create();
  const listeners = new Map<string, () => void>();
  const trace: TraceEntry[] = [];
  const record = (entry: TraceEntry) => trace.push(structuredClone(entry));
  try {
    for (const step of scenario.steps) {
      record({ kind: "enter", ...step });
      try {
        const result = execute(step, engine, listeners, record);
        record({
          kind: "return",
          op: step.op,
          ...(result === undefined ? {} : { result }),
        });
      } catch (error) {
        record({
          kind: "error",
          op: step.op,
          message: error instanceof Error ? error.message : String(error),
        });
      }
    }
  } finally {
    engine.dispose();
  }
  return trace;
}

function execute(
  step: SelectionOperation,
  engine: SelectionEngine,
  listeners: Map<string, () => void>,
  record: (entry: TraceEntry) => void,
): unknown {
  switch (step.op) {
    case "read":
      return engine.read(step.target!);
    case "stats":
      return engine.stats();
    case "send":
      engine.send(step.event ?? { type: "inc", by: step.by ?? 0 });
      return;
    case "unsubscribe":
      listeners.get(step.listener!)?.();
      return;
    case "subscribe":
      listeners.get(step.listener!)?.();
      listeners.set(
        step.listener!,
        engine.subscribe(step.target!, (value) => {
          record({
            kind: "select",
            listener: step.listener,
            target: step.target,
            value,
          });
          if (step.fail) throw new Error(step.fail);
          if (step.sendBy !== undefined && value.value === step.whenValue) {
            engine.send({ type: "inc", by: step.sendBy });
            record({
              kind: "nested-return",
              listener: step.listener,
              value: engine.read(step.target!),
            });
          }
        }),
      );
  }
}
