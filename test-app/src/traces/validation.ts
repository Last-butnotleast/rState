import type { TraceEntry } from "../engines/contract";
import type {
  ValidationEngine,
  ValidationFactory,
  ValidationOperation,
  ValidationScenario,
} from "../engines/validation";
import { validationFailure } from "../engines/validation";

export function runValidation(
  scenario: ValidationScenario,
  create: ValidationFactory,
): TraceEntry[] {
  const trace: TraceEntry[] = [];
  const record = (entry: TraceEntry) => trace.push(structuredClone(entry));
  let engine: ValidationEngine;
  try {
    engine = create(scenario.configuration, record);
  } catch (failure) {
    record({ kind: "createError", ...validationFailure(failure) });
    return trace;
  }
  record({ kind: "initial", snapshot: engine.snapshot() });
  engine.onAny((message) => record({ kind: "emit", message }));
  try {
    for (const step of scenario.steps) {
      record({ kind: "enter", ...step });
      try {
        const result = execute(step, engine, record);
        record({
          kind: "return",
          op: step.op,
          ...(result === undefined ? {} : { result }),
        });
      } catch (failure) {
        record({
          kind: "error",
          op: step.op,
          ...validationFailure(failure),
          snapshot: engine.snapshot(),
        });
      }
    }
  } finally {
    engine.dispose();
  }
  return trace;
}

function execute(
  step: ValidationOperation,
  engine: ValidationEngine,
  record: (entry: TraceEntry) => void,
): unknown {
  switch (step.op) {
    case "metadata":
      return engine.metadata();
    case "snapshot":
      return engine.snapshot();
    case "history":
      return engine.history();
    case "send":
      return engine.send(step.event!);
    case "can":
      return engine.can(step.event!);
    case "transition":
      return engine.transition(step.event!);
    case "reset":
      return engine.reset();
    case "resetWithCount":
      return engine.resetWithCount(step.count!);
    case "undo":
      return engine.undo();
    case "redo":
      return engine.redo();
    case "subscribe":
      engine.subscribe((snapshot) => record({ kind: "observe", snapshot }));
      return;
    case "inspect":
      engine.inspect((inspection) =>
        record({ kind: "inspect", ...inspection }),
      );
  }
}
