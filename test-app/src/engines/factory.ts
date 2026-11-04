import type { Engine, Operation } from "./contract";

export type FactoryInput = { count: number };

export interface FixtureFactory {
  create(input: FactoryInput): Engine;
  dispose(): void;
}

export type FactoryOperation = Omit<Operation, "op"> & {
  store: string;
  op: Operation["op"] | "initial" | "subscribeSelected";
  field?: "count" | "total";
};

export type FactoryScenario = {
  name: string;
  instances: { name: string; input: FactoryInput }[];
  steps: FactoryOperation[];
};
