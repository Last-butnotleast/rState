import type {
  Engine,
  Operation,
  Recorder,
  Scenario,
  ScenarioName,
  TraceEntry,
} from "../engines/contract";

export type EngineFactory = (
  scenario: ScenarioName,
  record: Recorder,
) => Engine;

export function runScenario(
  scenario: Scenario,
  create: EngineFactory,
): TraceEntry[] {
  const trace: TraceEntry[] = [];
  const record: Recorder = (entry) => trace.push(structuredClone(entry));
  const engine = create(scenario.scenario, record);
  const listeners = new Map<string, () => void>();
  engine.onAny((message) => record({ kind: "emit", message }));
  record({ kind: "initial", snapshot: engine.snapshot() });
  try {
    for (const step of scenario.steps) {
      record({
        kind: "enter",
        op: step.op,
        ...(step.event ? { event: step.event } : {}),
      });
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
  step: Operation,
  engine: Engine,
  listeners: Map<string, () => void>,
  record: Recorder,
): unknown {
  switch (step.op) {
    case "send":
      engine.send(step.event!);
      return;
    case "can":
      return engine.can(step.event!);
    case "transition":
      return engine.transition(step.event!);
    case "snapshot":
      return engine.snapshot();
    case "unsubscribe":
      listeners.get(step.listener!)?.();
      return;
    case "subscribe": {
      listeners.get(step.listener!)?.();
      let nested = false;
      const unsubscribe = engine.subscribe((snapshot) => {
        record({ kind: "observe", listener: step.listener, snapshot });
        if (step.fail) throw new Error(step.fail);
        if (step.send && !nested && snapshot.context.count === step.whenCount) {
          nested = true;
          engine.send(step.send);
        }
      });
      listeners.set(step.listener!, unsubscribe);
    }
  }
}
