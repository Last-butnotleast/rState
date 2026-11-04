import { createStore } from "@reference/store";
import type { Engine, Recorder, ScenarioName } from "./contract";
import { referenceConfig } from "./reference-config";
import { wrapReference } from "./reference-wrapper";

export function createReference(
  scenario: ScenarioName,
  record: Recorder,
): Engine {
  return wrapReference(createStore(referenceConfig(scenario, record)));
}
