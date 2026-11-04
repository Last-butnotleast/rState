import type { Engine, Recorder, TraceEntry } from "../engines/contract";
import type {
  FactoryOperation,
  FactoryScenario,
  FixtureFactory,
} from "../engines/factory";
import { createReferenceFactory } from "../engines/reference-factory";
import { createRustFactory } from "../engines/rust-factory";
import { compareTraces } from "./compare";

type Instance = { engine: Engine; listeners: Map<string, () => void> };

export function compareFactoryScenario(scenario: FactoryScenario) {
  return compareTraces(
    runFactoryScenario(scenario, createRustFactory),
    runFactoryScenario(scenario, createReferenceFactory),
  );
}

export function runFactoryScenario(
  scenario: FactoryScenario,
  create: () => FixtureFactory,
): TraceEntry[] {
  const factory = create();
  const instances = new Map<string, Instance>();
  const trace: TraceEntry[] = [];
  let disposed = false;
  const record: Recorder = (entry) => trace.push(structuredClone(entry));
  try {
    for (const { name, input } of scenario.instances) {
      const engine = factory.create(input);
      instances.set(name, { engine, listeners: new Map() });
      engine.onAny((message) => record({ kind: "emit", store: name, message }));
      record({ kind: "initial", store: name, snapshot: engine.snapshot() });
    }
    factory.dispose();
    disposed = true;
    for (const step of scenario.steps) {
      const instance = instances.get(step.store);
      if (!instance) throw new Error(`Unknown factory instance: ${step.store}`);
      record({ kind: "enter", ...step });
      try {
        const result = execute(step, instance, record);
        record({
          kind: "return",
          store: step.store,
          op: step.op,
          ...(result === undefined ? {} : { result }),
        });
      } catch (error) {
        record({
          kind: "error",
          store: step.store,
          op: step.op,
          message: error instanceof Error ? error.message : String(error),
          snapshot: instance.engine.snapshot(),
        });
      }
    }
  } finally {
    for (const { engine } of instances.values()) engine.dispose();
    if (!disposed) factory.dispose();
  }
  return trace;
}

function execute(
  step: FactoryOperation,
  { engine, listeners }: Instance,
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
    case "initial":
      return engine.initialSnapshot();
    case "unsubscribe":
      listeners.get(step.listener!)?.();
      return;
    case "subscribe":
    case "subscribeSelected": {
      listeners.get(step.listener!)?.();
      const callback = (value: unknown) =>
        record({
          kind: step.op === "subscribe" ? "observe" : "select",
          store: step.store,
          listener: step.listener,
          value,
        });
      const unsubscribe =
        step.op === "subscribe"
          ? engine.subscribe(callback)
          : engine.subscribeSelected(step.field!, callback);
      listeners.set(step.listener!, unsubscribe);
    }
  }
}
