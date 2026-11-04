import { createStoreLogic } from "@reference/store";
import type { FixtureFactory } from "./factory";
import { referenceConfig } from "./reference-config";
import { wrapReference } from "./reference-wrapper";

export function createReferenceFactory(): FixtureFactory {
  const config = referenceConfig("composition", () => {});
  const factory = createStoreLogic({
    on: config.on,
    context: (input) => ({ count: input.count, items: [], total: 0 }),
  });
  return {
    create: (input) => wrapReference(factory.createStore(input)),
    dispose: () => {},
  };
}
