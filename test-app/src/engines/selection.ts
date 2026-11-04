import type { Event } from "./contract";

export type SelectionTarget = "source" | "mapped" | "chain";
export type SelectedValue = { value: number | string; id: number };
export type SelectionStats = Record<SelectionTarget, number>;

export interface SelectionEngine {
  read(target: SelectionTarget): SelectedValue;
  stats(): SelectionStats;
  send(event: Event): void;
  subscribe(
    target: SelectionTarget,
    callback: (value: SelectedValue) => void,
  ): () => void;
  dispose(): void;
}

export type SelectionOperation = {
  op: "read" | "stats" | "send" | "subscribe" | "unsubscribe";
  target?: SelectionTarget;
  listener?: string;
  event?: Event;
  by?: number;
  sendBy?: number;
  whenValue?: number | string;
  fail?: string;
};

export type SelectionScenario = {
  name: string;
  steps: SelectionOperation[];
};
