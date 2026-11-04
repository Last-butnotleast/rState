import counter from "../../../conformance/scenarios/counter.json";
import composition from "../../../conformance/scenarios/composition.json";
import ordering from "../../../conformance/scenarios/ordering.json";
import type { Event, Scenario, ScenarioName } from "../engines/contract";

export const scenarios = [counter, composition, ordering] as Scenario[];

export const actions: Record<ScenarioName, { label: string; event: Event }[]> =
  {
    counter: [
      { label: "Increment", event: { type: "inc", by: 1 } },
      { label: "Keep", event: { type: "keep" } },
      { label: "Equal replacement", event: { type: "equal" } },
    ],
    composition: [
      { label: "Add item · 8", event: { type: "add", price: 8 } },
      { label: "Add item · 12", event: { type: "add", price: 12 } },
    ],
    ordering: [
      { label: "Run nested dispatch", event: { type: "start" } },
      { label: "Bump", event: { type: "bump", by: 2 } },
    ],
  };
