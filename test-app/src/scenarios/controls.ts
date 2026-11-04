import controls from "../../../conformance/scenarios/controls.json";
import persistence from "../../../conformance/scenarios/persistence.json";
import type { ControlScenario } from "../engines/controls";

export const controlsScenario = controls as ControlScenario;
export const persistenceScenario = persistence as ControlScenario;
