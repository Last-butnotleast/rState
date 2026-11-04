import type { Recorder, TraceEntry } from "../engines/contract";
import type {
  ControlEngine,
  ControlFactory,
  ControlOperation,
  ControlScenario,
  StorageAdapter,
} from "../engines/controls";
import { createReferenceControls } from "../engines/reference-controls";
import { createRustControls } from "../engines/rust-controls";
import { compareTraces } from "./compare";

export function compareControlScenario(scenario: ControlScenario) {
  return compareTraces(
    runControlScenario(scenario, createRustControls),
    runControlScenario(scenario, createReferenceControls),
  );
}

export function runControlScenario(
  scenario: ControlScenario,
  create: ControlFactory,
): TraceEntry[] {
  const trace: TraceEntry[] = [];
  const record: Recorder = (entry) => trace.push(structuredClone(entry));
  const stored = new Map<string, string>();
  if (scenario.stored && scenario.configuration.persistence) {
    stored.set(
      scenario.configuration.persistence.name,
      JSON.stringify(scenario.stored),
    );
  }
  const storage: StorageAdapter = {
    getItem: (name) => {
      const value = stored.get(name) ?? null;
      record({ kind: "storageRead", name, value: decode(value) });
      return value;
    },
    setItem: (name, value) => {
      record({ kind: "storageWrite", name, value: decode(value) });
      stored.set(name, value);
    },
    removeItem: (name) => {
      record({ kind: "storageClear", name });
      stored.delete(name);
    },
  };
  const engine = create(scenario.configuration, record, storage);
  const listeners = new Map<string, () => void>();
  engine.onAny((message) => record({ kind: "emit", message }));
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
      } catch (failure) {
        record({
          kind: "error",
          op: step.op,
          message: failure instanceof Error ? failure.message : String(failure),
          snapshot: engine.snapshot(),
        });
      }
    }
    record({
      kind: "stored",
      values: Object.fromEntries(
        [...stored].map(([name, value]) => [name, decode(value)]),
      ),
    });
  } finally {
    engine.dispose();
  }
  return trace;
}

function execute(
  step: ControlOperation,
  engine: ControlEngine,
  listeners: Map<string, () => void>,
  record: Recorder,
): unknown {
  switch (step.op) {
    case "send":
      return engine.send(step.event!);
    case "reset":
      return engine.reset();
    case "resetWithCount":
      return engine.resetWithCount(step.count!);
    case "undo":
      return engine.undo();
    case "redo":
      return engine.redo();
    case "snapshot":
      return engine.snapshot();
    case "initial":
      return engine.initialSnapshot();
    case "history":
      return engine.history();
    case "save":
      return engine.save();
    case "clearStorage":
      return engine.clearStorage();
    case "persistenceError":
      return engine.persistenceError();
    case "unsubscribe":
      return listeners.get(step.listener!)?.();
    case "subscribe":
    case "subscribeSelected":
    case "inspect": {
      const listener = step.listener!;
      listeners.get(listener)?.();
      const callback = (value: unknown) =>
        record({ kind: step.op, listener, value });
      const unsubscribe =
        step.op === "subscribe"
          ? engine.subscribe(callback)
          : step.op === "inspect"
            ? engine.inspect(callback)
            : engine.subscribeSelected(step.field!, callback);
      listeners.set(listener, unsubscribe);
    }
  }
}

function decode(value: string | null): unknown {
  return value === null ? null : JSON.parse(value);
}
