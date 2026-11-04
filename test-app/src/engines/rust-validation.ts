import { FixtureValidation } from "../generated/rstate_wasm";
import type { ValidationFactory } from "./validation";
import { wrapRustStore } from "./rust";

export const createRustValidation: ValidationFactory = (
  configuration,
  record,
  storage,
) => {
  if (configuration.persistence && !storage)
    throw new Error("Storage adapter required");
  const fixture = new FixtureValidation(
    JSON.stringify(configuration),
    (serialized: string) => record(JSON.parse(serialized)),
    storage ? (name: string) => storage.getItem(name) : undefined,
    storage
      ? (name: string, value: string) => storage.setItem(name, value)
      : undefined,
    storage ? (name: string) => storage.removeItem(name) : undefined,
  );
  const base = wrapRustStore(fixture.base());
  let disposed = false;
  return {
    ...base,
    metadata: () => JSON.parse(fixture.metadata()),
    history: () => JSON.parse(fixture.history()),
    reset: () => fixture.reset(),
    resetWithCount: (count) => fixture.reset_with_count(BigInt(count)),
    undo: () => fixture.undo(),
    redo: () => fixture.redo(),
    dispose: () => {
      if (disposed) return;
      base.dispose();
      fixture.free();
      disposed = true;
    },
  };
};
