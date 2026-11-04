import fixture from "../../../conformance/scenarios/combine.json";
import type { CombineMode, CombineScenario } from "../engines/combine";

export function combineScenario(mode: CombineMode): CombineScenario {
  return { ...fixture, mode } as CombineScenario;
}
