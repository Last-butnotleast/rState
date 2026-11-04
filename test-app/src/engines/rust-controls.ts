import { FixtureControls } from "../generated/rstate_wasm";
import type { ControlFactory } from "./controls";
import { wrapRustStore } from "./rust";

export const createRustControls: ControlFactory = (
  configuration,
  record,
  storage,
) => {
  if (configuration.persistence && !storage)
    throw new Error("Storage adapter required");
  const controls = new FixtureControls(
    JSON.stringify(configuration),
    (serialized: string) => record(JSON.parse(serialized)),
    storage ? (name: string) => storage.getItem(name) : undefined,
    storage
      ? (name: string, value: string) => storage.setItem(name, value)
      : undefined,
    storage ? (name: string) => storage.removeItem(name) : undefined,
    (serialized: string) =>
      record({ kind: "persistError", message: JSON.parse(serialized) }),
  );
  const engine = wrapRustStore(controls.base());
  let disposed = false;
  return {
    ...engine,
    reset: () => controls.reset(),
    resetWithCount: (count) => controls.reset_with_count(BigInt(count)),
    undo: () => controls.undo(),
    redo: () => controls.redo(),
    history: () => JSON.parse(controls.history()),
    save: () => controls.save(),
    clearStorage: () => controls.clear_storage(),
    persistenceError: () => controls.persistence_error() ?? null,
    dispose: () => {
      if (disposed) return;
      engine.dispose();
      controls.free();
      disposed = true;
    },
  };
};
