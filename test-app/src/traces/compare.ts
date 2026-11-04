import { createReference } from "../engines/reference";
import { createRust } from "../engines/rust";
import type { Scenario } from "../engines/contract";
import { runScenario } from "./run";

export function compareScenario(scenario: Scenario) {
  const rust = runScenario(scenario, createRust);
  const reference = runScenario(scenario, createReference);
  const differences = Array.from(
    { length: Math.max(rust.length, reference.length) },
    (_, index) => index,
  ).filter((index) => canonical(rust[index]) !== canonical(reference[index]));
  return { equal: differences.length === 0, differences, rust, reference };
}

function canonical(value: unknown): string {
  if (Array.isArray(value)) return `[${value.map(canonical).join(",")}]`;
  if (value !== null && typeof value === "object") {
    return `{${Object.entries(value)
      .sort(([left], [right]) => left.localeCompare(right))
      .map(([key, item]) => `${JSON.stringify(key)}:${canonical(item)}`)
      .join(",")}}`;
  }
  return JSON.stringify(value);
}
