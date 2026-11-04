import type {
  CombineEngine,
  CombineOperation,
  CombineScenario,
} from "../engines/combine";
import type { TraceEntry } from "../engines/contract";
import { createReferenceCombine } from "../engines/reference-combine";
import { createRustCombine } from "../engines/rust-combine";
import { compareTraces } from "./compare";

export function compareCombineScenario(scenario: CombineScenario) {
  return compareTraces(
    runCombineScenario(scenario, createRustCombine),
    runCombineScenario(scenario, createReferenceCombine),
  );
}

export function runCombineScenario(
  scenario: CombineScenario,
  create: typeof createRustCombine,
): TraceEntry[] {
  const engine = create(scenario.mode);
  const trace: TraceEntry[] = [];
  const listeners = new Map<string, () => void>();
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
  step: CombineOperation,
  engine: CombineEngine,
  listeners: Map<string, () => void>,
  record: (entry: TraceEntry) => void,
): unknown {
  switch (step.op) {
    case "read":
      return engine.read(step.target!);
    case "stats":
      return engine.stats();
    case "send":
      engine.send(
        step.input ?? "left",
        step.event ?? { type: "inc", by: step.by ?? 0 },
      );
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
            engine.send(step.sendInput ?? "left", {
              type: "inc",
              by: step.sendBy,
            });
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
