import { FixtureFactory as WasmFactory } from "../generated/rstate_wasm";
import type { FixtureFactory } from "./factory";
import { wrapRustStore } from "./rust";

export function createRustFactory(): FixtureFactory {
  const factory = new WasmFactory();
  return {
    create: (input) => wrapRustStore(factory.create(JSON.stringify(input))),
    dispose: () => factory.free(),
  };
}
