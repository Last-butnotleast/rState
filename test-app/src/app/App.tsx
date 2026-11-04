import { useState } from "react";
import type { ScenarioName } from "../engines/contract";
import { scenarios } from "../scenarios/catalog";
import { LabDisplay } from "./LabDisplay";
import { useLab } from "./useLab";
import { FactoryDisplay } from "./factory/FactoryDisplay";
import { useFactory } from "./factory/useFactory";
import { SelectionDisplay } from "./selection/SelectionDisplay";
import { useSelection } from "./selection/useSelection";

export function App() {
  const [name, setName] = useState<ScenarioName>("counter");
  const scenario = scenarios.find((value) => value.scenario === name)!;
  const { session, view, error } = useLab(scenario);
  const factory = useFactory();
  const selection = useSelection();
  return (
    <LabDisplay
      scenario={scenario}
      scenarios={scenarios}
      view={view}
      error={error}
      onScenario={setName}
      onSend={(event) => session?.send(event)}
      onCompare={() => session?.compare()}
    >
      <FactoryDisplay
        view={factory.view}
        error={factory.error}
        onSend={(instance, event) => factory.session?.send(instance, event)}
        onCompare={() => factory.session?.compare()}
      />
      <SelectionDisplay
        view={selection.view}
        error={selection.error}
        onIncrement={() => selection.session?.increment()}
        onCompare={() => selection.session?.compare()}
      />
    </LabDisplay>
  );
}
