import { useState } from "react";
import type { ScenarioName } from "../engines/contract";
import { scenarios } from "../scenarios/catalog";
import { LabDisplay } from "./LabDisplay";
import { useLab } from "./useLab";

export function App() {
  const [name, setName] = useState<ScenarioName>("counter");
  const scenario = scenarios.find((value) => value.scenario === name)!;
  const { session, view, error } = useLab(scenario);
  return (
    <LabDisplay
      scenario={scenario}
      scenarios={scenarios}
      view={view}
      error={error}
      onScenario={setName}
      onSend={(event) => session?.send(event)}
      onCompare={() => session?.compare()}
    />
  );
}
