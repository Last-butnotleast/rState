import type { Event } from "./contract";
import type { SelectedValue } from "./selection";

export type CombineMode = "independent" | "diamond" | "duplicate";
export type CombineInput = "left" | "right";
export type CombineTarget = CombineInput | "combined" | "chain";
export type CombineStats = Record<CombineTarget | "source", number>;

export interface CombineEngine {
  read(target: CombineTarget): SelectedValue;
  stats(): CombineStats;
  send(input: CombineInput, event: Event): void;
  subscribe(
    target: CombineTarget,
    callback: (value: SelectedValue) => void,
  ): () => void;
  dispose(): void;
}

export type CombineOperation = {
  op: "read" | "stats" | "send" | "subscribe" | "unsubscribe";
  target?: CombineTarget;
  input?: CombineInput;
  listener?: string;
  event?: Event;
  by?: number;
  sendInput?: CombineInput;
  sendBy?: number;
  whenValue?: number | string;
  fail?: string;
};

export type CombineScenario = {
  name: string;
  mode: CombineMode;
  steps: CombineOperation[];
};
